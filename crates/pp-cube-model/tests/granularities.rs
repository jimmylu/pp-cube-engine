//! Declared granularities and the approximate distinct count in the contracts.

use pp_cube_model::{
    load_json, load_query, load_yaml, validate_yaml, CustomGranularity, Granularity, MeasureType,
};

const MODEL: &str = r#"
cubes:
  - name: sales
    sql_table: dw.sales_dtl
    data_source: warehouse
    dimensions:
      - { name: sale_id, sql: "{CUBE}.sale_id", type: number, primary_key: true }
      - name: sold_at
        sql: "{CUBE}.sold_at"
        type: time
        granularities:
          - { name: half_year, interval: 6 month, origin: "2026-01-01" }
          - { name: fiscal_year, interval: 1 year, offset: 3 month }
    measures:
      - { name: buyers_approx, type: count_distinct_approx, sql: "{CUBE}.buyer_id" }
"#;

// scenario: B1-MODEL-002
#[test]
fn granularities_and_the_approximate_count_load_as_written_from_both_formats() {
    let model = load_yaml(MODEL).unwrap();
    let cube = &model.cubes[0];
    assert_eq!(
        cube.dimensions[1].granularities,
        [
            CustomGranularity {
                name: "half_year".into(),
                interval: "6 month".into(),
                origin: Some("2026-01-01".into()),
                offset: None,
            },
            CustomGranularity {
                name: "fiscal_year".into(),
                interval: "1 year".into(),
                origin: None,
                offset: Some("3 month".into()),
            },
        ]
    );
    assert_eq!(
        cube.measures[0].measure_type,
        MeasureType::CountDistinctApprox
    );
    let json = serde_json::to_string(&model).unwrap();
    assert_eq!(load_json(&json).unwrap(), model);
    // A dimension without granularities is written without the field.
    let plain = serde_json::to_value(&cube.dimensions[0]).unwrap();
    assert!(plain.get("granularities").is_none());
    assert!(
        validate_yaml(MODEL).is_empty(),
        "{:?}",
        validate_yaml(MODEL)
    );
}

fn issues(original: &str, replacement: &str) -> Vec<(String, String)> {
    assert!(MODEL.contains(original), "{original}");
    let issues = validate_yaml(&MODEL.replace(original, replacement));
    issues.into_iter().map(|i| (i.path, i.rule_id)).collect()
}

// scenario: B1-VALID-003
#[test]
fn granularity_rules_are_reported_at_their_fields() {
    let at = "/cubes/0/dimensions/1/granularities/0";
    let written = r#"{ name: half_year, interval: 6 month, origin: "2026-01-01" }"#;
    for (replacement, path, rule) in [
        // Values outside the contract.
        (
            r#"{ name: Half Year, interval: 6 month, origin: "2026-01-01" }"#,
            "/name",
            "type",
        ),
        (
            r#"{ name: month, interval: 6 month, origin: "2026-01-01" }"#,
            "/name",
            "type",
        ),
        (
            r#"{ name: half_year, interval: 6 months, origin: "2026-01-01" }"#,
            "/interval",
            "type",
        ),
        (
            r#"{ name: half_year, interval: 12 hour, origin: "2026-01-01" }"#,
            "/interval",
            "type",
        ),
        (
            r#"{ name: half_year, interval: 6 month, origin: "January" }"#,
            "/origin",
            "type",
        ),
        (
            r#"{ name: half_year, interval: 1 month, offset: "soon" }"#,
            "/offset",
            "type",
        ),
        (
            r#"{ name: half_year, origin: "2026-01-01" }"#,
            "/interval",
            "required",
        ),
        (
            r#"{ name: half_year, interval: 6 month, origin: "2026-01-01", sql: "x" }"#,
            "/sql",
            "unknown-field",
        ),
        // Several units are counted from an origin: without one it would be the
        // start of the year the query happens to be compiled in.
        ("{ name: half_year, interval: 6 month }", "", "granularity"),
        (
            "{ name: half_year, interval: 6 month, offset: 1 month }",
            "",
            "granularity",
        ),
        (
            r#"{ name: half_year, interval: 1 month, origin: "2026-01-15", offset: 1 day }"#,
            "",
            "granularity",
        ),
        // Declared twice.
        (
            r#"{ name: fiscal_year, interval: 6 month, origin: "2026-01-01" }"#,
            "",
            "duplicate",
        ),
    ] {
        let expected = if rule == "duplicate" {
            vec![(
                "/cubes/0/dimensions/1/granularities/1/name".to_owned(),
                "duplicate-name".to_owned(),
            )]
        } else {
            vec![(format!("{at}{path}"), rule.to_owned())]
        };
        assert_eq!(issues(written, replacement), expected, "{replacement}");
    }
    // One unit needs neither; an origin alone moves it.
    for replacement in [
        "{ name: half_year, interval: 1 month }",
        r#"{ name: half_year, interval: 1 month, origin: "2026-01-15T06:00:00" }"#,
    ] {
        assert!(issues(written, replacement).is_empty(), "{replacement}");
    }
    // Only time has periods.
    assert_eq!(
        issues(
            r#"{ name: sale_id, sql: "{CUBE}.sale_id", type: number, primary_key: true }"#,
            r#"{ name: sale_id, sql: "{CUBE}.sale_id", type: number, primary_key: true, granularities: [{ name: tens, interval: 1 day }] }"#
        ),
        [(
            "/cubes/0/dimensions/0/granularities".to_owned(),
            "granularity".to_owned()
        )]
    );
    // An estimate has no window of rows to be exact over.
    assert_eq!(
        issues(
            r#"type: count_distinct_approx, sql: "{CUBE}.buyer_id""#,
            r#"type: count_distinct_approx, sql: "{CUBE}.buyer_id", rolling_window: { trailing: 7 day }"#
        ),
        [(
            "/cubes/0/measures/0/rolling_window".to_owned(),
            "window".to_owned()
        )]
    );
}

// scenario: B1-QUERY-001
#[test]
fn query_keeps_a_declared_granularity_by_name_and_ungrouped_as_written() {
    let text = r#"{"ungrouped": true, "dimensions": ["sales.sale_id"],
        "timeDimensions": [{"dimension": "sales.sold_at", "granularity": "half_year"},
                           {"dimension": "sales.sold_at", "granularity": "month"}]}"#;
    let query = load_query(text).unwrap();
    assert_eq!(query.ungrouped, Some(true));
    assert_eq!(
        query.time_dimensions[0].granularity,
        Some(Granularity::Custom("half_year".into()))
    );
    assert_eq!(
        query.time_dimensions[1].granularity,
        Some(Granularity::Month)
    );
    // Written back by name, and read again as the same query.
    let written = serde_json::to_value(&query).unwrap();
    assert_eq!(written["timeDimensions"][0]["granularity"], "half_year");
    assert_eq!(written["timeDimensions"][1]["granularity"], "month");
    assert_eq!(load_query(&written.to_string()).unwrap(), query);
    // No feature of this query is refused before compilation.
    assert!(query.unsupported_features().is_empty());
    // What cannot be a name is still refused where it is written.
    for name in ["Half Year", "half-year", "6month", ""] {
        let text = format!(
            r#"{{"timeDimensions": [{{"dimension": "sales.sold_at", "granularity": "{name}"}}]}}"#
        );
        assert!(load_query(&text).is_err(), "{name}");
    }
}
