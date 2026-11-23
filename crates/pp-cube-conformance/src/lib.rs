//! Conformance: live-compiled execution cases for engine tiers.
//!
//! Nothing here executes SQL. A tier requests the manifest of one dialect,
//! loads each case's fixture on a real engine and compares the result with the
//! expectation written beside the case.

pub mod cases;
mod doris;
mod presto;

use std::sync::Arc;

use pp_cube_compile::{compile, CompileContext, DialectId, Parameter};
use serde_json::{json, Value};

/// The manifest layout version an execution tier must recognise.
pub const SCHEMA: u32 = 1;

/// One query, the data it runs on and the result it must return.
/// Expected columns and rows are literals chosen independently of the compiler.
#[derive(Debug, Clone)]
pub struct Case {
    pub name: &'static str,
    /// The scenario this case supports, such as `B0-DORIS-010`.
    pub scenario: &'static str,
    /// Idempotent dialect statements that leave exactly this case's data.
    pub fixture: &'static [&'static str],
    /// Cube query JSON.
    pub query: String,
    pub timezone: &'static str,
    /// Cube query JSON whose filters are the security filters; empty for none.
    pub security: &'static str,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
}

/// An expected timestamp cell at midnight of `day`.
pub fn midnight(day: &str) -> Value {
    json!({ "timestamp": format!("{day}T00:00:00") })
}

/// Compiles every case of the named dialect now and returns its manifest.
/// Fails, without a partial document, for a dialect that has no cases.
pub fn manifest(dialect: &str) -> Result<String, String> {
    match dialect {
        "doris" => render(
            DialectId::Doris,
            dialect,
            MODEL,
            &dialect_cases(doris::fixture, doris::scenario),
        ),
        "presto" => render(
            DialectId::Presto,
            dialect,
            MODEL,
            &dialect_cases(presto::fixture, presto::scenario),
        ),
        other => Err(format!("unknown dialect `{other}`")),
    }
}

/// The model every shared case is compiled against.
const MODEL: &str = include_str!("../fixtures/sales.yaml");

/// The shared cases with one dialect's fixtures and scenario ids.
fn dialect_cases(
    fixture: fn(cases::Fixture) -> &'static [&'static str],
    scenario: fn(cases::Group) -> &'static str,
) -> Vec<Case> {
    cases::declared()
        .into_iter()
        .map(|case| Case {
            name: case.name,
            scenario: scenario(case.group),
            fixture: fixture(case.fixture),
            query: case.query,
            timezone: case.timezone,
            security: case.security,
            columns: case.columns,
            rows: case.rows,
        })
        .collect()
}

/// Compiles `cases` against `model_yaml`. Any case that cannot be compiled or
/// represented fails the whole manifest and is named in the error.
/// The manifest's `session` is the union of the session properties the compile
/// results declare, so a tier sets exactly what the kernel says its SQL needs.
pub fn render(
    dialect: DialectId,
    label: &str,
    model_yaml: &str,
    cases: &[Case],
) -> Result<String, String> {
    if cases.is_empty() {
        return Err(format!("dialect `{label}` has no execution cases"));
    }
    let model =
        Arc::new(pp_cube_model::load_yaml(model_yaml).map_err(|error| format!("model: {error}"))?);
    let mut names = std::collections::BTreeSet::new();
    let mut session = serde_json::Map::new();
    let mut entries = Vec::with_capacity(cases.len());
    for case in cases {
        let fail = |reason: String| format!("case `{}`: {reason}", case.name);
        if !names.insert(case.name) {
            return Err(fail("duplicate name".into()));
        }
        if case.scenario.is_empty() || case.fixture.is_empty() || case.columns.is_empty() {
            return Err(fail("scenario, fixture and columns are required".into()));
        }
        let query = pp_cube_model::load_query(&case.query).map_err(|e| fail(e.to_string()))?;
        let security_filters = if case.security.is_empty() {
            Vec::new()
        } else {
            pp_cube_model::load_query(case.security)
                .map_err(|e| fail(e.to_string()))?
                .filters
        };
        let context = CompileContext {
            request_id: String::new(),
            dialect,
            timezone: case.timezone.into(),
            security_filters,
        };
        let compiled = compile(&model, &query, &context).map_err(|e| fail(e.to_string()))?;
        let params = compiled
            .parameters
            .iter()
            .map(parameter)
            .collect::<Result<Vec<_>, _>>()
            .map_err(fail)?;
        for property in compiled.session_properties {
            if let Some(other) = session.insert(property.name.clone(), json!(property.value)) {
                if other != json!(property.value) {
                    return Err(fail(format!(
                        "session property `{}` has two values",
                        property.name
                    )));
                }
            }
        }
        entries.push(json!({
            "name": case.name,
            "scenario": case.scenario,
            "fixture": case.fixture,
            "sql": compiled.sql,
            "params": params,
            "columns": case.columns,
            "rows": case.rows,
        }));
    }
    let document = json!({
        "schema": SCHEMA,
        "dialect": label,
        "session": session,
        "cases": entries,
    });
    Ok(format!("{document:#}\n"))
}

/// Every parameter keeps an explicit type so a tier never guesses a binding.
fn parameter(parameter: &Parameter) -> Result<Value, String> {
    Ok(match parameter {
        Parameter::Text(value) => json!({ "type": "text", "value": value }),
        Parameter::Bool(value) => json!({ "type": "bool", "value": value }),
        Parameter::Number(value) if value.is_finite() => {
            json!({ "type": "number", "value": value })
        }
        Parameter::Number(_) => return Err("a non-finite number has no manifest type".into()),
        Parameter::Null => json!({ "type": "null" }),
    })
}
