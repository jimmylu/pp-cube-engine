//! The compile facade: model, query and context in; SQL and parameters out.
//!
//! Nothing here executes SQL, reads the clock or performs IO.

use std::fmt;
use std::sync::Arc;

use pp_cube_adapter::{plan, PlanError};
use pp_cube_dialect::dialect;
use pp_cube_model::{Filter, Model, Query};

pub use pp_cube_adapter::Parameter;
pub use pp_cube_dialect::DialectId;

/// What a compilation needs besides the model and the query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileContext {
    pub dialect: DialectId,
    /// An IANA name. A query that names its own timezone is compiled in that one.
    pub timezone: String,
    /// Restrictions appended as top-level AND entries beside user filters.
    /// An empty list adds no restriction; empty groups at any depth fail.
    /// Inputs are never mutated. Use dimension filters for row restrictions.
    pub security_filters: Vec<Filter>,
}

/// One SELECT in the context's dialect and the values of its placeholders, in order.
#[derive(Debug, Clone, PartialEq)]
pub struct Compiled {
    pub sql: String,
    pub parameters: Vec<Parameter>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    /// The caller can correct it; the message names the member or the rule.
    Query(String),
    /// The kernel or the planner failed. `detail` is for the operator's log;
    /// it can name internals and is not part of the message.
    Internal { detail: String },
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Query(message) => f.write_str(message),
            Self::Internal { .. } => f.write_str("the query could not be compiled"),
        }
    }
}

impl std::error::Error for CompileError {}

/// Compiles one query.
pub fn compile(
    model: &Arc<Model>,
    query: &Query,
    context: &CompileContext,
) -> Result<Compiled, CompileError> {
    for filter in &context.security_filters {
        check_security_filter(filter)?;
    }
    let mut query = query.clone();
    query
        .filters
        .extend(context.security_filters.iter().cloned());
    let dialect = dialect(context.dialect).map_err(|error| CompileError::Internal {
        detail: error.to_string(),
    })?;
    match plan(model, &query, dialect, &context.timezone) {
        Ok(planned) => Ok(Compiled {
            sql: planned.sql,
            parameters: planned.parameters,
        }),
        Err(PlanError::User(message)) => Err(CompileError::Query(message)),
        Err(PlanError::Internal(detail)) => Err(CompileError::Internal { detail }),
    }
}

fn check_security_filter(filter: &Filter) -> Result<(), CompileError> {
    if let Filter::And(children) | Filter::Or(children) = filter {
        if children.is_empty() {
            return Err(CompileError::Query(
                "A security filter group must not be empty".to_owned(),
            ));
        }
        for child in children {
            check_security_filter(child)?;
        }
    }
    Ok(())
}
