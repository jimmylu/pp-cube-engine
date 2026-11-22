//! The compile facade: model, query and context in; SQL and parameters out.
//!
//! Nothing here executes SQL, reads the clock or performs IO.

mod fingerprint;

use std::fmt;
use std::sync::Arc;

use pp_cube_adapter::{plan, PlanError};
use pp_cube_dialect::dialect;
use pp_cube_model::{validate_model, Filter, Model, Query, ValidationIssue};

pub use pp_cube_adapter::{OutputColumn, Parameter, UsedCube};
pub use pp_cube_dialect::DialectId;

/// What a compilation needs besides the model and the query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileContext {
    /// Correlation only; excluded from result identity.
    pub request_id: String,
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
    pub columns: Vec<OutputColumn>,
    pub used_cubes: Vec<UsedCube>,
    /// SHA-256 of canonical typed-model content, query, semantic context and build identity.
    /// Raw YAML/JSON formatting and request_id are excluded.
    pub fingerprint: [u8; 32],
    pub sql: String,
    pub parameters: Vec<Parameter>,
}

/// Caller-facing errors contain no planner SQL or physical source diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    /// The bundle is invalid; issues match the public model validator report.
    Model(Vec<ValidationIssue>),
    /// The caller can correct it; the message names the member or the rule.
    Query(String),
    /// A recognized request exceeds B0 or the selected dialect's implemented envelope.
    Unsupported(String),
    /// The kernel or planner failed. The existing detail field is retained for
    /// source compatibility and carries a safe category, never raw diagnostics.
    Internal { detail: String },
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Model(_) => f.write_str("the model could not be validated"),
            Self::Query(message) | Self::Unsupported(message) => f.write_str(message),
            Self::Internal { .. } => f.write_str("the query could not be compiled"),
        }
    }
}

impl std::error::Error for CompileError {}

/// Compiles one query after validating the complete model.
/// Model errors take precedence over query and security-filter errors.
pub fn compile(
    model: &Arc<Model>,
    query: &Query,
    context: &CompileContext,
) -> Result<Compiled, CompileError> {
    let issues = validate_model(model);
    if !issues.is_empty() {
        return Err(CompileError::Model(issues));
    }
    if let Some(feature) = query.unsupported_b0_features().first() {
        return Err(CompileError::Unsupported(format!(
            "The recognized Cube feature `{feature}` is outside B0"
        )));
    }
    for filter in &context.security_filters {
        check_security_filter(filter)?;
    }
    if context.dialect == DialectId::Doris {
        check_doris(model, query, context)?;
    }
    let fingerprint = fingerprint::fingerprint(model, query, context)?;
    let mut query = query.clone();
    query
        .filters
        .extend(context.security_filters.iter().cloned());
    let dialect = dialect(context.dialect).map_err(|error| CompileError::Internal {
        detail: error.to_string(),
    })?;
    match plan(model, &query, dialect, &context.timezone) {
        Ok(planned) => Ok(Compiled {
            columns: planned.columns,
            used_cubes: planned.used_cubes,
            fingerprint,
            sql: planned.sql,
            parameters: planned.parameters,
        }),
        Err(error) => Err(error.into()),
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

impl From<PlanError> for CompileError {
    fn from(error: PlanError) -> Self {
        match error {
            PlanError::User(message) => Self::Query(message),
            PlanError::Unsupported(message) => Self::Unsupported(message),
            PlanError::Internal(_) => Self::Internal {
                detail: "The planner could not compile the query".into(),
            },
        }
    }
}

// scenario: B0-RESULT-002
#[test]
fn internal_mapping_keeps_operator_detail_out_of_display() {
    let detail = "SELECT hidden_column FROM hidden_schema.hidden_table";
    let error = CompileError::from(PlanError::Internal(detail.into()));
    assert_eq!(error.to_string(), "the query could not be compiled");
    assert!(!format!("{error:?}").contains("hidden_"));
    assert_eq!(
        error,
        CompileError::Internal {
            detail: "The planner could not compile the query".into()
        }
    );
    assert_eq!(
        CompileError::from(PlanError::User("planner rule".into())),
        CompileError::Query("planner rule".into())
    );
}

// Intentionally conservative until the remaining Doris matrix is implemented.
fn check_doris(model: &Model, query: &Query, ctx: &CompileContext) -> Result<(), CompileError> {
    use pp_cube_model::{DimensionType, Operator};
    fn supported(filter: &Filter) -> bool {
        match filter {
            Filter::And(fs) | Filter::Or(fs) => fs.iter().all(supported),
            Filter::Condition(c) => matches!(
                c.operator,
                Operator::Contains
                    | Operator::NotContains
                    | Operator::StartsWith
                    | Operator::NotStartsWith
                    | Operator::EndsWith
                    | Operator::NotEndsWith
                    | Operator::Equals
                    | Operator::NotEquals
                    | Operator::Gt
                    | Operator::Gte
                    | Operator::Lt
                    | Operator::Lte
                    | Operator::Set
                    | Operator::NotSet
            ),
        }
    }
    if model.cubes.iter().any(|c| {
        c.dimensions
            .iter()
            .any(|d| d.dimension_type == DimensionType::Time)
    }) || !query.time_dimensions.is_empty()
        || (query.offset.is_some() && query.limit.is_none())
        || !query
            .filters
            .iter()
            .chain(&ctx.security_filters)
            .all(supported)
    {
        return Err(CompileError::Unsupported("Doris compilation currently requires models without time dimensions, scalar/pattern filters and a limit when offset is present".into()));
    }
    Ok(())
}
