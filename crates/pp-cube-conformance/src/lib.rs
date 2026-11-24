//! Conformance: live-compiled execution cases for engine tiers.
//!
//! Nothing here executes SQL. A tier requests the manifest of one dialect,
//! loads each case's fixture on a real engine and compares the result with the
//! expectation written beside the case.

pub mod cases;
mod doris;
mod druid;
pub mod insurance;
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
    emit(dialect, false)
}

/// The manifest with what a comparison against Cube Core needs besides the
/// compiled SQL: the model text, and each case's query in Cube's own format with
/// its timezone and the security filters that were applied to it.
pub fn manifest_with_sources(dialect: &str) -> Result<String, String> {
    emit(dialect, true)
}

fn emit(dialect: &str, sources: bool) -> Result<String, String> {
    let (id, cases) = match dialect {
        "doris" => (
            DialectId::Doris,
            dialect_cases(doris::fixture, doris::scenario),
        ),
        "presto" => (
            DialectId::Presto,
            dialect_cases(presto::fixture, presto::scenario),
        ),
        "druid" => {
            // Druid answers part of the cases: those its dialect compiles. What it
            // refuses is left out here and listed by `druid_support`.
            let model = druid::model(model_text());
            let mut cases = dialect_cases(druid::fixture, druid::scenario);
            let support = druid_support()?;
            cases.retain(|case| {
                support
                    .iter()
                    .any(|(name, outcome)| *name == case.name && outcome == "compiled")
            });
            let document = document(DialectId::Druid, dialect, &model, &cases, sources)?;
            return Ok(format!("{document:#}\n"));
        }
        other => return Err(format!("unknown dialect `{other}`")),
    };
    let document = document(id, dialect, model_text(), &cases, sources)?;
    Ok(format!("{document:#}\n"))
}

/// How each shared case fares on Druid: `compiled`, or `unsupported` with the
/// kernel's reason. Any other failure is an error: the manifest fails closed.
pub fn druid_support() -> Result<Vec<(&'static str, String)>, String> {
    let model = druid::model(model_text());
    let model = Arc::new(pp_cube_model::load_yaml(&model).map_err(|error| error.to_string())?);
    let mut support = Vec::new();
    for case in cases::declared() {
        let fail = |reason: String| format!("case `{}`: {reason}", case.name);
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
            dialect: DialectId::Druid,
            timezone: case.timezone.into(),
            security_filters,
        };
        let outcome = match compile(&model, &query, &context) {
            Ok(_) => "compiled".to_owned(),
            Err(pp_cube_compile::CompileError::Unsupported(reason)) => {
                format!("unsupported: {reason}")
            }
            Err(error) => return Err(fail(error.to_string())),
        };
        support.push((case.name, outcome));
    }
    Ok(support)
}

/// The retail model of the feature cases.
const MODEL: &str = include_str!("../fixtures/sales.yaml");
/// The property-insurance model of the domain scenario.
const INSURANCE_MODEL: &str = include_str!("../fixtures/insurance.yaml");

/// The model every shared case is compiled against: the cubes and the views of
/// both scenarios in one document, as one deployment would hold them.
pub fn model_text() -> &'static str {
    static TEXT: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
        let parts = |text: &'static str| {
            let (cubes, views) = text.split_once("\nviews:\n").unwrap_or((text, ""));
            (cubes.trim_start_matches("cubes:\n"), views)
        };
        let (retail, insurance) = (parts(MODEL), parts(INSURANCE_MODEL));
        format!(
            "cubes:\n{}\n{}\nviews:\n{}{}",
            retail.0.trim_end(),
            insurance.0.trim_end(),
            retail.1,
            insurance.1
        )
    });
    &TEXT
}

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
    document(dialect, label, model_yaml, cases, false).map(|document| format!("{document:#}\n"))
}

fn document(
    dialect: DialectId,
    label: &str,
    model_yaml: &str,
    cases: &[Case],
    sources: bool,
) -> Result<Value, String> {
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
        let mut entry = json!({
            "name": case.name,
            "scenario": case.scenario,
            "fixture": case.fixture,
            "sql": compiled.sql,
            "params": params,
            "columns": case.columns,
            "rows": case.rows,
        });
        if sources {
            // The query as the case wrote it, not a re-serialization of what was loaded.
            let written: Value =
                serde_json::from_str(&case.query).map_err(|e| fail(e.to_string()))?;
            let filters: Value = if case.security.is_empty() {
                json!([])
            } else {
                serde_json::from_str::<Value>(case.security).map_err(|e| fail(e.to_string()))?
                    ["filters"]
                    .take()
            };
            entry["query"] = written;
            entry["timezone"] = json!(case.timezone);
            entry["security_filters"] = filters;
        }
        entries.push(entry);
    }
    let mut document = json!({
        "schema": SCHEMA,
        "dialect": label,
        "session": session,
        "cases": entries,
    });
    if sources {
        let data_sources: std::collections::BTreeSet<&str> = model
            .cubes
            .iter()
            .map(|cube| cube.data_source.as_str())
            .collect();
        document["model"] = json!(model_yaml);
        document["data_sources"] = json!(data_sources);
    }
    Ok(document)
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

/// The shared model padded with copies of its sales cube up to `cubes` cubes,
/// for measuring how compilation scales with model size.
pub fn padded_model(cubes: usize) -> Result<pp_cube_model::Model, String> {
    let mut model = pp_cube_model::load_yaml(MODEL).map_err(|error| error.to_string())?;
    let template = model
        .cubes
        .iter()
        .find(|cube| cube.name == "sales")
        .cloned()
        .ok_or("the shared model has no sales cube")?;
    for index in model.cubes.len()..cubes {
        let mut cube = template.clone();
        cube.name = format!("sales_copy_{index}");
        cube.sql_table = Some(format!("dw.sales_copy_{index}"));
        model.cubes.push(cube);
    }
    Ok(model)
}

/// The shared cases as loaded queries with their timezone and security filters.
#[allow(clippy::type_complexity)]
pub fn bench_cases(
) -> Result<Vec<(pp_cube_model::Query, String, Vec<pp_cube_model::Filter>)>, String> {
    cases::declared()
        .into_iter()
        .map(|case| {
            let query = pp_cube_model::load_query(&case.query).map_err(|e| e.to_string())?;
            let security = if case.security.is_empty() {
                Vec::new()
            } else {
                pp_cube_model::load_query(case.security)
                    .map_err(|e| e.to_string())?
                    .filters
            };
            Ok((query, case.timezone.to_owned(), security))
        })
        .collect()
}
