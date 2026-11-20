//! Implements the bridge interfaces the upstream planner requires, over the
//! contract types, and drives the planner through them.
//!
//! The planner is used as it is at the commit pinned by the `vendor/cube`
//! submodule. Nothing under `vendor/` is edited in this repository, and no
//! other crate touches the planner's interfaces.

mod expression;
mod options;
mod schema;
mod tools;

use std::rc::Rc;
use std::sync::Arc;

use cubenativeutils::{CubeError, CubeErrorCauseType};
use cubesqlplanner::cube_bridge::base_query_options::FilterValue;
use cubesqlplanner::planner::{
    ModelContext, QueryModelContext, QueryPropertiesCompiler, RequestOptions, State,
    TopLevelPlanner,
};
use pp_cube_dialect::Dialect;
use pp_cube_model::{Model, Query};

use crate::options::Options;
use crate::schema::Evaluator;
use crate::tools::{Joins, Opaque, Tools};

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
    pub sql: String,
    pub parameters: Vec<Parameter>,
}

/// Why the planner produced no SQL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanError {
    /// The query or the model is at fault; the message names the member or rule.
    User(String),
    /// The kernel or the planner is at fault.
    Internal(String),
}

impl From<CubeError> for PlanError {
    fn from(error: CubeError) -> Self {
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

/// Plans `query` against `model` and renders it in `dialect`.
pub fn plan(
    model: &Arc<Model>,
    query: &Query,
    dialect: &'static Dialect,
) -> Result<Planned, PlanError> {
    let evaluator = Rc::new(Evaluator::new(model.clone()));
    let joins = Rc::new(Joins);
    let tools = Rc::new(Tools::new(evaluator.clone(), dialect, joins.clone()));
    let security_context = Rc::new(Opaque);
    let options = Rc::new(Options::new(
        query,
        evaluator.clone(),
        tools.clone(),
        joins.clone(),
        security_context.clone(),
    ));
    let state = State::try_new(
        evaluator,
        security_context,
        tools,
        joins,
        None,
        false,
        false,
        None,
        None,
        None,
        QueryModelContext::new(Rc::new(ModelContext::new()), None, false),
        RequestOptions::default(),
    )?;
    let request = QueryPropertiesCompiler::new(state.clone()).build(options)?;
    let (raw_sql, _) = TopLevelPlanner::new(request, state.clone()).plan()?;
    let templates = state.plan_sql_templates(false)?;
    let (sql, values) = state.build_sql_and_params(&raw_sql, &templates)?;
    let parameters = values
        .into_iter()
        .map(|value| match value {
            FilterValue::Str(text) => Parameter::Text(text),
            FilterValue::Bool(flag) => Parameter::Bool(flag),
            FilterValue::Num(number) => Parameter::Number(number),
            FilterValue::Null => Parameter::Null,
        })
        .collect();
    Ok(Planned { sql, parameters })
}
