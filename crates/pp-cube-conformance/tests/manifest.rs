use std::process::{Command, Output};

use pp_cube_compile::DialectId;
use pp_cube_conformance::{render, Case};
use serde_json::Value;

fn emit(dialect: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_execution-cases"))
        .arg(dialect)
        .output()
        .expect("start the emitter")
}

// scenario: B0-EXEC-001
#[test]
fn doris_manifest_is_stable_and_complete() {
    let first = emit("doris");
    assert!(first.status.success());
    assert_eq!(first.stdout, emit("doris").stdout);
    let manifest: Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(manifest["schema"], 1);
    assert_eq!(manifest["dialect"], "doris");
    let cases = manifest["cases"].as_array().unwrap();
    let scenario = |id: &str| cases.iter().filter(|c| c["scenario"] == id).count();
    assert_eq!(
        (
            cases.len(),
            scenario("B0-DORIS-010"),
            scenario("B0-DORIS-002"),
            scenario("B0-DORIS-011")
        ),
        (40, 13, 26, 1)
    );
    let mut names = std::collections::BTreeSet::new();
    for case in cases {
        let name = case["name"].as_str().unwrap();
        assert!(names.insert(name), "{name} is declared twice");
        assert!(!case["fixture"].as_array().unwrap().is_empty(), "{name}");
        assert!(
            case["sql"].as_str().unwrap().starts_with("SELECT"),
            "{name}"
        );
        let params = case["params"].as_array().unwrap();
        assert_eq!(
            case["sql"].as_str().unwrap().matches('?').count(),
            params.len(),
            "{name}"
        );
        for param in params {
            assert_eq!(param["type"], "text", "{name}");
            assert!(param["value"].is_string(), "{name}");
        }
        let columns = case["columns"].as_array().unwrap();
        let rows = case["rows"].as_array().unwrap();
        // Only the page past the last group expects no rows.
        assert_eq!(rows.is_empty(), name == "page_past_last_group", "{name}");
        for row in rows {
            assert_eq!(row.as_array().unwrap().len(), columns.len(), "{name}");
        }
    }
    assert_eq!(cases[0]["rows"][0][0]["timestamp"], "2026-10-01T00:00:00");
}

fn golden_text(dialect: &str) -> String {
    let manifest: Value =
        serde_json::from_str(&pp_cube_conformance::manifest(dialect).unwrap()).unwrap();
    let mut text = String::new();
    for case in manifest["cases"].as_array().unwrap() {
        let values: Vec<_> = case["params"]
            .as_array()
            .unwrap()
            .iter()
            .map(|param| param["value"].to_string())
            .collect();
        text.push_str(&format!(
            "-- {}\n{}\n-- parameters: [{}]\n\n",
            case["name"].as_str().unwrap(),
            case["sql"].as_str().unwrap(),
            values.join(", ")
        ));
    }
    text
}

// scenario: B0-EXEC-003
// scenario: B0-DORIS-001
#[test]
fn doris_cases_match_their_goldens() {
    insta::assert_snapshot!("doris_cases", golden_text("doris"));
}

// scenario: B0-EXEC-004
// scenario: B0-PRESTO-017
#[test]
fn presto_shares_the_doris_cases_and_matches_its_goldens() {
    let manifest = |dialect: &str| -> Value {
        serde_json::from_str(&pp_cube_conformance::manifest(dialect).unwrap()).unwrap()
    };
    let (doris, presto) = (manifest("doris"), manifest("presto"));
    let (doris_cases, presto_cases) = (
        doris["cases"].as_array().unwrap(),
        presto["cases"].as_array().unwrap(),
    );
    assert_eq!(presto_cases.len(), 40);
    assert_eq!(doris_cases.len(), presto_cases.len());
    // Same questions and the same answers on both engines; only SQL and fixtures differ.
    for (doris, presto) in doris_cases.iter().zip(presto_cases) {
        let name = presto["name"].as_str().unwrap();
        for field in ["name", "columns", "rows"] {
            assert_eq!(doris[field], presto[field], "{name}: {field}");
        }
        assert_eq!(presto["scenario"], "B0-PRESTO-003", "{name}");
        assert_ne!(doris["sql"], presto["sql"], "{name}");
        assert!(presto["sql"].as_str().unwrap().contains('"'), "{name}");
        assert!(
            presto["fixture"][0].as_str().unwrap().contains("memory.dw"),
            "{name}"
        );
    }
    // Doris needs no session property; Presto pagination needs exactly this one.
    assert_eq!(doris["session"], serde_json::json!({}));
    assert_eq!(
        presto["session"],
        serde_json::json!({"offset_clause_enabled": "true"})
    );
    let paginated: Vec<_> = presto_cases
        .iter()
        .filter(|case| case["sql"].as_str().unwrap().contains("\nOFFSET "))
        .map(|case| case["name"].as_str().unwrap())
        .collect();
    assert_eq!(paginated, ["order_limit_offset", "page_past_last_group"]);
    insta::assert_snapshot!("presto_cases", golden_text("presto"));
}

// scenario: B0-EXEC-002
#[test]
fn unusable_requests_emit_no_manifest() {
    for (dialect, named) in [("druid", "druid"), ("oracle", "oracle")] {
        let output = emit(dialect);
        assert!(!output.status.success(), "{dialect}");
        assert!(output.stdout.is_empty(), "{dialect}");
        assert!(String::from_utf8_lossy(&output.stderr).contains(named));
    }
    let model = "cubes:\n  - name: sales\n    sql_table: dw.sales_dtl\n    data_source: warehouse\n    measures:\n      - { name: amount, type: sum, sql: \"{CUBE}.amount\" }\n";
    let good = Case {
        name: "good",
        scenario: "B0-DORIS-010",
        fixture: &["SELECT 1"],
        query: r#"{"measures":["sales.amount"]}"#.into(),
        timezone: "UTC",
        security: "",
        columns: vec!["sales__amount".into()],
        rows: vec![],
    };
    assert!(render(
        DialectId::Doris,
        "doris",
        model,
        std::slice::from_ref(&good)
    )
    .is_ok());
    let error = render(DialectId::Doris, "doris", model, &[]).unwrap_err();
    assert!(error.contains("`doris` has no execution cases"), "{error}");
    let broken = Case {
        name: "broken",
        query: r#"{"measures":["sales.missing"]}"#.into(),
        ..good.clone()
    };
    let error = render(DialectId::Doris, "doris", model, &[good.clone(), broken]).unwrap_err();
    assert!(error.contains("`broken`"), "{error}");
    let error = render(DialectId::Doris, "doris", model, &[good.clone(), good]).unwrap_err();
    assert!(error.contains("duplicate"), "{error}");
}
