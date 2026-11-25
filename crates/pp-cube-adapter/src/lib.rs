//! Implements the bridge interfaces the upstream planner requires, over the
//! contract types, and drives the planner through them.
//!
//! The planner is used as it is at the commit pinned by the `vendor/cube`
//! submodule. Nothing under `vendor/` is edited in this repository, and no
//! other crate touches the planner's interfaces.

mod expression;
mod granularities;
mod result;
pub use result::{OutputColumn, UsedCube};
mod joins;
mod options;
mod schema;
mod tools;
mod windows;

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
use pp_cube_model::{Filter, Model, Query};

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

impl PlanError {
    fn upstream(error: CubeError, query: &Query, stage: &str) -> Self {
        match error.cause {
            CubeErrorCauseType::User => {
                // Translate audited pinned-upstream rule templates; never echo their
                // interpolated SQL/values. Error class comes from cause, not text.
                let rule = if error.message.starts_with("Member ")
                    && error.message.contains("levels deep")
                {
                    "Member reference chains must stay within the planner resolution depth limit"
                } else if error
                    .message
                    .starts_with("Could not resolve nonexistent time ")
                {
                    "Local time must exist in the effective timezone"
                } else if error.message.starts_with("Can't parse date:") {
                    "Date values must be valid absolute dates"
                } else if error
                    .message
                    .starts_with("Unsupported timestamp precision:")
                {
                    "Timestamp precision must be supported by the planner"
                } else if error.message == "Expected one parameter but nothing found" {
                    "The filter requires a parameter"
                } else {
                    "The request must satisfy the planner's query rules"
                };
                fn filters<'a>(items: &'a [Filter], members: &mut Vec<&'a str>) {
                    for filter in items {
                        match filter {
                            Filter::Condition(c) => members.push(&c.member),
                            Filter::And(items) | Filter::Or(items) => filters(items, members),
                        }
                    }
                }
                let mut members: Vec<&str> = query
                    .measures
                    .iter()
                    .chain(&query.dimensions)
                    .chain(&query.segments)
                    .map(String::as_str)
                    .collect();
                filters(&query.filters, &mut members);
                members.extend(query.time_dimensions.iter().map(|t| t.dimension.as_str()));
                members.extend(query.order.iter().flatten().map(|o| o.member.as_str()));
                members.sort_unstable();
                members.dedup();
                Self::User(format!("{rule}; members {members:?}; stage {stage}"))
            }
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
    if let Some(feature) = query.unsupported_features().first() {
        return Err(PlanError::Unsupported(format!(
            "The recognized Cube feature `{feature}` is outside B0"
        )));
    }
    let timezone = query.timezone.as_deref().unwrap_or(timezone);
    let zone: Tz = timezone
        .parse()
        .map_err(|_| PlanError::User(format!("Timezone `{timezone}` is not known")))?;
    let evaluator = Rc::new(Evaluator::new(model.clone()));
    evaluator.check(query).map_err(PlanError::local)?;
    windows::check(model, query, dialect)?;
    granularities::check(model, query, dialect)?;
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
    // Detail rows are the rows of one SELECT. Where a measure needs an aggregation
    // of its own, the planner groups part of the statement after all.
    // The engine cannot run more than one SELECT of this shape: say so, with what
    // usually causes it, rather than hand over SQL it rejects.
    if dialect.refuses("multi-stage") && sql.trim_start().starts_with("WITH") {
        return Err(PlanError::Unsupported(format!(
            "The query needs more than one SELECT, which {} does not support: measures of \
             several cubes, or a measure over rows a join repeats",
            dialect.name()
        )));
    }
    if query.ungrouped == Some(true) && sql.trim_start().starts_with("WITH") {
        return Err(PlanError::Unsupported(
            "An ungrouped query cannot take measures that need an aggregation of their own: \
             measures of several cubes, over rows a join repeats, rolling or shifted"
                .into(),
        ));
    }
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
fn upstream_internal_cause_retains_adapter_diagnostics() {
    let detail = "SELECT hidden_column FROM hidden_schema.hidden_table";
    assert_eq!(
        PlanError::upstream(
            CubeError::internal(detail.into()),
            &Query::default(),
            "planning"
        ),
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

// scenario: B0-AGG-003
#[test]
fn direct_planning_rejects_recognized_boundary_features_before_building_options() {
    let model = Arc::new(
        pp_cube_model::load_yaml(include_str!(
            "../../pp-cube-compile/tests/fixtures/retail.yaml"
        ))
        .unwrap(),
    );
    let dialect = pp_cube_dialect::dialect(pp_cube_dialect::DialectId::Presto).unwrap();
    for text in [
        r#"{"timeDimensions":[{"dimension":"sales.sold_at","compareDateRange":[]}]}"#,
        r#"{"timeDimensions":[{"dimension":"sales.sold_at","granularity":"second"}]}"#,
        r#"{"timeDimensions":[{"dimension":"sales.sold_at","granularity":"minute"}]}"#,
        r#"{"timeDimensions":[{"dimension":"sales.sold_at","granularity":"hour"}]}"#,
    ] {
        let query = pp_cube_model::load_query(text).unwrap();
        assert!(
            matches!(
                plan(&model, &query, dialect, "UTC"),
                Err(PlanError::Unsupported(_))
            ),
            "{text}"
        );
    }
}
