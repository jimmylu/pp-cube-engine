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
    let names: Vec<_> = cases.iter().map(|c| c["name"].as_str().unwrap()).collect();
    assert_eq!(
        names,
        [
            "day",
            "week",
            "month",
            "quarter",
            "year",
            "inDateRange",
            "notInDateRange",
            "beforeDate",
            "beforeOrOnDate",
            "afterDate",
            "afterOrOnDate",
            "security",
            "dst"
        ]
    );
    for case in cases {
        let name = case["name"].as_str().unwrap();
        assert_eq!(case["scenario"], "B0-DORIS-010", "{name}");
        assert!(!case["fixture"].as_array().unwrap().is_empty(), "{name}");
        assert!(
            case["sql"].as_str().unwrap().starts_with("SELECT"),
            "{name}"
        );
        let params = case["params"].as_array().unwrap();
        assert!(!params.is_empty(), "{name}");
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
        assert!(!rows.is_empty(), "{name}");
        for row in rows {
            assert_eq!(row.as_array().unwrap().len(), columns.len(), "{name}");
        }
    }
    // The security range survives a user OR; the boundary rows decide the total.
    assert_eq!(cases[11]["params"].as_array().unwrap().len(), 4);
    assert_eq!(cases[0]["rows"][0][0]["timestamp"], "2026-10-01T00:00:00");
}

// scenario: B0-EXEC-002
#[test]
fn unusable_requests_emit_no_manifest() {
    for (dialect, named) in [("presto", "presto"), ("oracle", "oracle")] {
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
