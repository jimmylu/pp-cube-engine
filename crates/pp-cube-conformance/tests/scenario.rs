//! The property-insurance scenario: what of a domain's questions the kernel compiles.

use std::sync::Arc;

use pp_cube_compile::{compile, CompileContext, CompileError, DialectId};
use pp_cube_conformance::{insurance, model_text};

/// How one surveyed query fares on one dialect.
fn outcome(model: &Arc<pp_cube_model::Model>, dialect: DialectId, query: &str) -> &'static str {
    let Ok(query) = pp_cube_model::load_query(query) else {
        return "not read";
    };
    let context = CompileContext {
        request_id: String::new(),
        dialect,
        timezone: "UTC".into(),
        security_filters: Vec::new(),
    };
    match compile(model, &query, &context) {
        Ok(_) => "compiled",
        Err(CompileError::Query(_)) => "query error",
        Err(CompileError::Unsupported(_)) => "unsupported",
        Err(CompileError::Model(_)) => "model error",
        Err(CompileError::Internal { .. }) => "internal",
    }
}

// scenario: B1-EXEC-001
#[test]
fn the_support_matrix_of_the_insurance_scenario_is_the_reviewed_one() {
    let text = model_text();
    assert!(pp_cube_model::validate_yaml(text).is_empty());
    let model = Arc::new(pp_cube_model::load_yaml(text).unwrap());
    let mut matrix = String::new();
    let mut compiled = Vec::new();
    for (name, query) in insurance::survey() {
        let presto = outcome(&model, DialectId::Presto, query);
        let doris = outcome(&model, DialectId::Doris, query);
        // A failure of the kernel is never an entry of the matrix.
        assert!(
            presto != "internal" && doris != "internal" && presto != "model error",
            "{name}"
        );
        if presto == "compiled" && doris == "compiled" {
            compiled.push(name);
        }
        matrix.push_str(&format!("{name:<34} presto: {presto:<12} doris: {doris}\n"));
    }
    insta::assert_snapshot!("insurance_support_matrix", matrix);
    // Every executed case is a surveyed query that compiles on both dialects,
    // and is in both manifests with the same expectation.
    let manifest = |dialect: &str| -> serde_json::Value {
        serde_json::from_str(&pp_cube_conformance::manifest(dialect).unwrap()).unwrap()
    };
    let (presto, doris) = (manifest("presto"), manifest("doris"));
    let find = |manifest: &serde_json::Value, name: &str| {
        let cases = manifest["cases"].as_array().unwrap();
        cases.iter().find(|case| case["name"] == name).cloned()
    };
    let executed = insurance::cases();
    assert_eq!(executed.len(), 13);
    for case in &executed {
        let surveyed = if case.name.starts_with("security") {
            "x_premium_paid_by_line"
        } else {
            case.name
        };
        assert!(compiled.contains(&surveyed), "{}", case.name);
        let (in_presto, in_doris) = (find(&presto, case.name), find(&doris, case.name));
        let (in_presto, in_doris) = (in_presto.unwrap(), in_doris.unwrap());
        for field in ["columns", "rows"] {
            assert_eq!(in_presto[field], in_doris[field], "{}: {field}", case.name);
        }
        assert!(!case.rows.is_empty(), "{}", case.name);
    }
}
