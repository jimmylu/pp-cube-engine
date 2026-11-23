use pp_cube_model::validate_json;

fn cube(sql: &str) -> String {
    serde_json::json!({"cubes":[{"name":"sales","data_source":"main","sql":sql}]}).to_string()
}

// scenario: B0-VALID-002
#[test]
fn malformed_cube_sql_is_located() {
    let issues = validate_json(&cube("SELECT FROM"));
    assert!(issues
        .iter()
        .any(|i| i.path == "/cubes/0/sql" && i.rule_id == "sql-syntax"));
}

// scenario: B0-VALID-002
#[test]
fn cube_sql_is_one_row_level_table_query() {
    for sql in [
        "SELECT * FROM sales; SELECT * FROM sales",
        "SELECT SUM(x) FROM sales",
        "SELECT DISTINCT x FROM sales",
        "SELECT row_number() OVER () FROM sales",
        "SELECT * FROM sales JOIN stores ON true",
        "SELECT * FROM (SELECT * FROM sales) s",
        "SELECT * FROM sales WHERE EXISTS (SELECT 1)",
        "SELECT (SELECT 1) FROM sales",
        "SELECT * FROM sales UNION SELECT * FROM sales",
        "WITH s AS (SELECT * FROM sales) SELECT * FROM s",
        "SELECT * FROM sales ORDER BY x",
        "SELECT * FROM sales LIMIT 1",
        "SELECT * FROM sales GROUP BY x",
        "SELECT coalesce(SUM(x),0) FROM sales",
        "DELETE FROM sales",
    ] {
        assert!(
            validate_json(&cube(sql))
                .iter()
                .any(|i| i.path == "/cubes/0/sql"),
            "{sql}"
        );
    }
    for sql in [
        "SELECT * FROM sales",
        "-- comment\n SELECT s.\"x\", lower(y) FROM sales AS s WHERE y > 0;",
        "SELECT 'SUM(x)' FROM sales /* comment */",
    ] {
        assert!(validate_json(&cube(sql)).is_empty(), "{sql}");
    }
}

fn model(expression: &str) -> serde_json::Value {
    serde_json::json!({"cubes":[{"name":"sales","data_source":"main","sql_table":"dw.sales",
        "dimensions":[{"name":"id","sql":"{CUBE}.id","type":"number","primary_key":true},
            {"name":"value","sql":expression,"type":"number"}],
        "measures":[{"name":"total","sql":"{CUBE}.amount","type":"sum"},
            {"name":"ratio","sql":"{total} / NULLIF({total},0)","type":"number"}]},
        {"name":"stores","data_source":"main","sql_table":"dw.stores",
        "dimensions":[{"name":"id","sql":"{CUBE}.id","type":"number","primary_key":true}]}]})
}

fn reports(v: &serde_json::Value) -> Vec<pp_cube_model::ValidationIssue> {
    let json = v.to_string();
    let report = validate_json(&json);
    assert_eq!(
        report,
        pp_cube_model::validate_yaml(&serde_norway::to_string(v).unwrap())
    );
    if let Ok(typed) = pp_cube_model::load_json(&json) {
        let before = typed.clone();
        assert_eq!(report, pp_cube_model::validate_model(&typed));
        assert_eq!(typed, before);
    }
    report
}

// scenario: B0-VALID-004
#[test]
fn references_preserve_source_and_do_not_collide() {
    for expression in [
        "{CUBE}.amount",
        "sales.amount",
        "dw.sales.amount",
        "\"sales\".\"amount\"",
        "{stores.id}",
        "{id}",
        "coalesce({CUBE}.amount, __pp_ref_0)",
        "'SUM(x)'",
        "lower('text')",
    ] {
        assert!(reports(&model(expression)).is_empty(), "{expression}");
    }
}

// scenario: B0-VALID-004
#[test]
fn row_expressions_reject_nested_and_indirect_aggregation() {
    for expression in [
        "SUM({CUBE}.amount)",
        "coalesce(pg_catalog.\"SuM\"(amount),0)",
        "{total}",
        "{ratio}",
        "{id} + {ratio}",
    ] {
        assert!(
            reports(&model(expression))
                .iter()
                .any(|i| i.path == "/cubes/0/dimensions/1/sql"
                    && i.rule_id == "expression-aggregate"),
            "{expression}"
        );
    }
    let mut v = model("{id}");
    v["cubes"][0]["dimensions"][0]["sql"] = "{value}".into();
    assert!(reports(&v).is_empty()); // Cycle traversal terminates; cycle policy is separate.
    v["cubes"][0]["segments"] = serde_json::json!([{"name":"active","sql":"{ratio} > 0"}]);
    v["cubes"][0]["measures"][0]["filters"] = serde_json::json!([{"sql":"MAX(amount) > 0"}]);
    assert_eq!(
        reports(&v)
            .iter()
            .filter(|i| i.rule_id == "expression-aggregate")
            .count(),
        2
    );
}

// scenario: B0-VALID-004
#[test]
fn physical_column_access_requires_the_owning_cube() {
    for expression in [
        "stores.secret_column",
        "dw.stores.secret_column",
        "\"stores\".\"secret_column\"",
        "{stores.id}.secret_column",
    ] {
        let issues = reports(&model(expression));
        assert!(
            issues.iter().any(|i| i.rule_id == "expression-column"),
            "{expression}"
        );
        assert!(issues.iter().all(|i| !i.message.contains("secret_column")));
    }
    let mut v = model("{id}");
    v["cubes"][0]["joins"] = serde_json::json!([{"name":"stores","relationship":"many_to_one","sql":"{CUBE}.id = {stores.id}"}]);
    assert!(reports(&v).is_empty());
    v["cubes"][0]["joins"][0]["sql"] = "sum({CUBE}.id) = {stores.id}".into();
    assert!(reports(&v)
        .iter()
        .any(|i| i.path == "/cubes/0/joins/0/sql" && i.rule_id == "expression-aggregate"));
}

// scenario: B0-VALID-003
#[test]
fn unsupported_features_accumulate_alongside_sql_faults() {
    for (key, value, path, rule) in [
        (
            "reduce_by",
            serde_json::json!([]),
            "/cubes/0/measures/0/reduce_by",
            "unknown-field",
        ),
        (
            "case",
            serde_json::json!({}),
            "/cubes/0/measures/0/case",
            "unknown-field",
        ),
        (
            "pre_aggregations",
            serde_json::json!([]),
            "/cubes/0/pre_aggregations",
            "unknown-field",
        ),
        (
            "reference",
            serde_json::json!("{absent}"),
            "/cubes/0/dimensions/1/sql",
            "reference",
        ),
        (
            "relationship",
            serde_json::json!("one_to_many"),
            "/cubes/0/joins/0/relationship",
            "type",
        ),
    ] {
        let mut v = model("{id}");
        match key {
            "pre_aggregations" => v["cubes"][0][key] = value,
            "reference" => v["cubes"][0]["dimensions"][1]["sql"] = value,
            "relationship" => {
                v["cubes"][0]["joins"] = serde_json::json!([{"name":"stores","sql":"{CUBE}.id = {stores.id}","relationship":value}])
            }
            _ => v["cubes"][0]["measures"][0][key] = value,
        }
        assert!(reports(&v)
            .iter()
            .any(|i| i.path == path && i.rule_id == rule));
        v["cubes"][1].as_object_mut().unwrap().remove("sql_table");
        v["cubes"][1]["sql"] = "SELECT SUM(id) FROM stores".into();
        let issues = reports(&v);
        assert!(issues.iter().any(|i| i.path == path && i.rule_id == rule));
        assert!(issues
            .iter()
            .any(|i| i.path == "/cubes/1/sql" && i.rule_id == "sql-construct"));
    }
}

// scenario: B0-VALID-004
#[test]
fn expressions_must_be_complete_and_row_level() {
    for expression in [
        "x; SELECT 1",
        "x y",
        "EXISTS(SELECT 1)",
        "row_number() OVER ()",
    ] {
        assert!(!reports(&model(expression)).is_empty(), "{expression}");
    }
}

// scenario: B0-VALID-002
#[test]
fn excessive_nesting_is_a_located_parse_failure() {
    let sql = format!("SELECT {}x{} FROM sales", "(".repeat(200), ")".repeat(200));
    assert!(validate_json(&cube(&sql))
        .iter()
        .any(|i| i.path == "/cubes/0/sql" && i.rule_id == "sql-syntax"));
}
