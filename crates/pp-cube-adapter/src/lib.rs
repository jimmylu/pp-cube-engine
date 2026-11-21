//! Implements the bridge interfaces the upstream planner requires, over the
//! contract types, and drives the planner through them.
//!
//! The planner is used as it is at the commit pinned by the `vendor/cube`
//! submodule. Nothing under `vendor/` is edited in this repository, and no
//! other crate touches the planner's interfaces.

mod expression;
mod result;
pub use result::{OutputColumn, UsedCube};
mod joins;
mod options;
mod schema;
mod tools;

use std::rc::Rc;
use std::sync::Arc;

use chrono_tz::Tz;
use cubenativeutils::{CubeError, CubeErrorCauseType};
use cubesqlplanner::cube_bridge::base_query_options::FilterValue;
use cubesqlplanner::planner::{
    ModelContext, QueryModelContext, QueryPropertiesCompiler, RequestOptions, State,
    TopLevelPlanner,
};
use pp_cube_dialect::Dialect;
use pp_cube_model::{Model, Query};

use crate::joins::Joins;
use crate::options::Options;
use crate::schema::Evaluator;
use crate::tools::{Opaque, Tools};

/// One value bound to a placeholder of the SQL.
#[derive(Debug, Clone, PartialEq)]
pub enum Parameter {
    Text(String),
    Bool(bool),
    Number(f64),
    Null,
}

/// The SQL of a query and the values of its placeholders, in order.
#[derive(Debug, Clone, PartialEq)]
pub struct Planned {
    pub columns: Vec<OutputColumn>,
    pub used_cubes: Vec<UsedCube>,
    pub sql: String,
    pub parameters: Vec<Parameter>,
}

/// Why the planner produced no SQL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanError {
    /// The query or the model is at fault; the message names the member or rule.
    User(String),
    /// A recognized request requires features beyond B0.
    Unsupported(String),
    /// Operator diagnostic; not a caller-safe message.
    Internal(String),
}

impl From<CubeError> for PlanError {
    fn from(error: CubeError) -> Self {
        match error.cause {
            CubeErrorCauseType::User => {
                Self::User("The query does not satisfy the planner rules".into())
            }
            _ => Self::Internal(error.message),
        }
    }
}

impl PlanError {
    fn upstream(error: CubeError, query: &Query, stage: &str) -> Self {
        match error.cause {
            CubeErrorCauseType::User => Self::User(format!(
                "Members {:?} / {:?} must satisfy planner {stage} rules",
                query.measures, query.dimensions
            )),
            _ => Self::Internal(error.message),
        }
    }

    // Only for audited local checks whose diagnostics name logical members/rules.
    fn local(error: CubeError) -> Self {
        match error.cause {
            CubeErrorCauseType::User => Self::User(error.message),
            _ => Self::Internal(error.message),
        }
    }
}

/// For the parts of the planner's interfaces that the kernel does not serve yet.
fn unavailable<T>(what: &str) -> Result<T, CubeError> {
    Err(CubeError::internal(format!(
        "{what} is not available in the kernel yet"
    )))
}

/// Plans `query` against `model` and renders it in `dialect`. `timezone` is
/// the one of the compile context; the query's own, if it has one, wins.
pub fn plan(
    model: &Arc<Model>,
    query: &Query,
    dialect: &'static Dialect,
    timezone: &str,
) -> Result<Planned, PlanError> {
    let timezone = query.timezone.as_deref().unwrap_or(timezone);
    let zone: Tz = timezone
        .parse()
        .map_err(|_| PlanError::User(format!("Timezone `{timezone}` is not known")))?;
    let evaluator = Rc::new(Evaluator::new(model.clone()));
    evaluator.check(query).map_err(PlanError::local)?;
    let joins = Rc::new(Joins::new(model.clone(), query)?);
    let tools = Rc::new(Tools::new(evaluator.clone(), dialect, zone, joins.clone()));
    let security_context = Rc::new(Opaque);
    let options = Rc::new(Options::new(
        query,
        zone.name(),
        evaluator.clone(),
        tools.clone(),
        joins.clone(),
        security_context.clone(),
    ));
    let state = joins.finish(
        State::try_new(
            evaluator,
            security_context,
            tools,
            joins.clone(),
            None,
            false,
            false,
            None,
            None,
            None,
            QueryModelContext::new(Rc::new(ModelContext::new()), None, false),
            RequestOptions::default(),
        ),
        query,
        "initialization",
    )?;
    let request = joins.finish(
        QueryPropertiesCompiler::new(state.clone()).build(options),
        query,
        "query validation",
    )?;
    let (raw_sql, _) = joins.finish(
        TopLevelPlanner::new(request.clone(), state.clone()).plan(),
        query,
        "planning",
    )?;
    let templates = joins.finish(state.plan_sql_templates(false), query, "template rendering")?;
    let (sql, values) = joins.finish(
        state.build_sql_and_params(&raw_sql, &templates),
        query,
        "parameter binding",
    )?;
    let parameters = values
        .into_iter()
        .map(|value| match value {
            FilterValue::Str(text) => Parameter::Text(text),
            FilterValue::Bool(flag) => Parameter::Bool(flag),
            FilterValue::Num(number) => Parameter::Number(number),
            FilterValue::Null => Parameter::Null,
        })
        .collect();
    let (columns, used_cubes) = result::metadata(&sql, request.all_members(false), model)?;
    Ok(Planned {
        sql,
        parameters,
        columns,
        used_cubes,
    })
}

// scenario: B0-RESULT-002
#[test]
fn upstream_messages_are_not_public_and_internal_causes_survive() {
    let detail = "SELECT hidden_column FROM hidden_schema.hidden_table";
    assert_eq!(
        PlanError::from(CubeError::user(detail.into())),
        PlanError::User("The query does not satisfy the planner rules".into())
    );
    assert_eq!(
        PlanError::from(CubeError::internal(detail.into())),
        PlanError::Internal(detail.into())
    );
}

// scenario: B0-RESULT-002
#[test]
fn upstream_user_conversion_names_logical_members_and_stage_without_raw_sql() {
    let query = Query {
        measures: vec!["sales.amount".into()],
        ..Query::default()
    };
    let error = PlanError::upstream(
        CubeError::user("SELECT hidden_column FROM hidden_table".into()),
        &query,
        "query validation",
    );
    let PlanError::User(message) = error else {
        panic!("expected user error")
    };
    assert!(message.contains("sales.amount"));
    assert!(message.contains("query validation"));
    assert!(!message.contains("hidden_"));
}
