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
        (85, 14, 70, 1)
    );
    let mut names = std::collections::BTreeSet::new();
    for case in cases {
        let name = case["name"].as_str().unwrap();
        assert!(names.insert(name), "{name} is declared twice");
        assert!(!case["fixture"].as_array().unwrap().is_empty(), "{name}");
        // One SELECT, or one preceded by common table expressions.
        let sql = case["sql"].as_str().unwrap();
        assert!(
            sql.starts_with("SELECT") || sql.starts_with(" WITH \n"),
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
    assert_eq!(presto_cases.len(), 85);
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
    assert_eq!(
        paginated,
        [
            "order_limit_offset",
            "page_past_last_group",
            "multi_fact_page",
            "ungrouped_joined_page"
        ]
    );
    insta::assert_snapshot!("presto_cases", golden_text("presto"));
}

// scenario: B0-EXEC-002
#[test]
fn unusable_requests_emit_no_manifest() {
    for (dialect, named) in [("trino", "trino"), ("oracle", "oracle")] {
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

// scenario: B0-EXEC-005
#[test]
fn the_sources_manifest_adds_what_cube_core_needs_and_changes_nothing_else() {
    let plain = emit("presto");
    let output = Command::new(env!("CARGO_BIN_EXE_execution-cases"))
        .args(["presto", "--sources"])
        .output()
        .expect("start the emitter");
    assert!(output.status.success());
    let mut sources: Value = serde_json::from_slice(&output.stdout).unwrap();
    // The model is the text the cases were compiled against, and it loads.
    let model = sources["model"].as_str().unwrap().to_owned();
    let loaded = pp_cube_model::load_yaml(&model).unwrap();
    assert!(loaded.cubes.iter().any(|cube| cube.name == "sales"));
    assert_eq!(sources["data_sources"], serde_json::json!(["warehouse"]));
    for case in sources["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        // The query is Cube's own format and loads as written.
        assert!(
            pp_cube_model::load_query(&case["query"].to_string()).is_ok(),
            "{name}"
        );
        assert!(case["timezone"].is_string(), "{name}");
        let filters = case["security_filters"].as_array().unwrap();
        assert_eq!(filters.is_empty(), !name.starts_with("security"), "{name}");
        let wrapped = serde_json::json!({ "filters": filters }).to_string();
        assert!(pp_cube_model::load_query(&wrapped).is_ok(), "{name}");
    }
    // Without the added fields it is exactly the plain manifest, so tiers that
    // read the plain manifest are unaffected.
    let document = sources.as_object_mut().unwrap();
    document.remove("model");
    document.remove("data_sources");
    for case in document["cases"].as_array_mut().unwrap() {
        let case = case.as_object_mut().unwrap();
        for field in ["query", "timezone", "security_filters"] {
            assert!(case.remove(field).is_some());
        }
    }
    let plain: Value = serde_json::from_slice(&plain.stdout).unwrap();
    assert_eq!(sources, plain);
    for arguments in [
        vec!["presto", "--other"],
        vec!["presto", "--sources", "extra"],
        vec![],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_execution-cases"))
            .args(&arguments)
            .output()
            .unwrap();
        assert!(
            !output.status.success() && output.stdout.is_empty(),
            "{arguments:?}"
        );
    }
}

// scenario: B0-EXEC-006
// scenario: B0-DRUID-001
// scenario: B0-DRUID-002
#[test]
fn druid_manifest_holds_the_cases_its_dialect_compiles_and_lists_the_rest() {
    let manifest = |dialect: &str| -> Value {
        serde_json::from_str(&pp_cube_conformance::manifest(dialect).unwrap()).unwrap()
    };
    assert_eq!(emit("druid").stdout, emit("druid").stdout);
    let (druid, presto) = (manifest("druid"), manifest("presto"));
    let (druid_cases, presto_cases) = (
        druid["cases"].as_array().unwrap(),
        presto["cases"].as_array().unwrap(),
    );
    // Every case is either in the manifest or refused with a reason.
    let support = pp_cube_conformance::druid_support().unwrap();
    assert_eq!(support.len(), presto_cases.len());
    let compiled: Vec<_> = support
        .iter()
        .filter(|(_, outcome)| outcome == "compiled")
        .map(|(name, _)| *name)
        .collect();
    let names: Vec<_> = druid_cases
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, compiled);
    assert_eq!(names.len(), 57);
    assert!(support
        .iter()
        .all(|(_, outcome)| outcome == "compiled" || outcome.starts_with("unsupported: ")));
    // The same questions and answers as the other engines, for the part it answers.
    for case in druid_cases {
        let name = case["name"].as_str().unwrap();
        let other = presto_cases.iter().find(|c| c["name"] == name).unwrap();
        for field in ["columns", "rows"] {
            assert_eq!(case[field], other[field], "{name}: {field}");
        }
        assert_eq!(case["scenario"], "B0-DRUID-004", "{name}");
        let sql = case["sql"].as_str().unwrap();
        // One SELECT on Druid's own schema, never the shared one.
        assert!(sql.starts_with("SELECT") && !sql.contains("dw."), "{name}");
        assert!(sql.contains("druid."), "{name}");
        assert_eq!(
            sql.matches('?').count(),
            case["params"].as_array().unwrap().len(),
            "{name}"
        );
        // A fixture replaces whole datasources with inline rows.
        for statement in case["fixture"].as_array().unwrap() {
            let statement = statement.as_str().unwrap();
            assert!(
                statement.starts_with("REPLACE INTO \"") && statement.contains("EXTERN("),
                "{name}"
            );
        }
    }
    // Exact distinct counts are asked of the engine for every statement.
    assert_eq!(
        druid["session"],
        serde_json::json!({"useApproximateCountDistinct": "false"})
    );
    let matrix: String = support
        .iter()
        .map(|(name, outcome)| format!("{name:<34} {outcome}\n"))
        .collect();
    insta::assert_snapshot!("druid_support", matrix);
    insta::assert_snapshot!("druid_cases", golden_text("druid"));
}

// scenario: B0-EXEC-007
#[test]
fn only_druid_cases_state_a_relation_to_cube_core_and_only_the_reviewed_ones() {
    let sources = |dialect: &str| -> Value {
        serde_json::from_str(&pp_cube_conformance::manifest_with_sources(dialect).unwrap()).unwrap()
    };
    let mut druid = sources("druid");
    let stated: Vec<(String, String)> = druid["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|case| {
            let relation = case.get("cube_core")?.as_str().unwrap().to_owned();
            Some((case["name"].as_str().unwrap().to_owned(), relation))
        })
        .collect();
    let stated: Vec<(&str, &str)> = stated
        .iter()
        .map(|(name, relation)| (name.as_str(), relation.as_str()))
        .collect();
    // Cube's Druid dialect has no approximate count and no date bin, and writes
    // an interval Druid rejects. Everything else must equal Cube Core's SQL.
    assert_eq!(
        stated,
        [
            ("approximate_distinct_count", "refuses"),
            ("granularity_of_two_days", "refuses"),
            ("granularity_of_two_quarters", "refuses"),
            ("granularity_with_offset", "differs"),
            ("uw_by_underwriting_year", "differs"),
        ]
    );
    assert!(sources("presto")["cases"]
        .as_array()
        .unwrap()
        .iter()
        .all(|case| case.get("cube_core").is_none()));
    // Without sources and relations it is the plain manifest.
    let document = druid.as_object_mut().unwrap();
    document.remove("model");
    document.remove("data_sources");
    for case in document["cases"].as_array_mut().unwrap() {
        let case = case.as_object_mut().unwrap();
        for field in ["query", "timezone", "security_filters"] {
            assert!(case.remove(field).is_some());
        }
        case.remove("cube_core");
    }
    let plain: Value =
        serde_json::from_str(&pp_cube_conformance::manifest("druid").unwrap()).unwrap();
    assert_eq!(druid, plain);
}
