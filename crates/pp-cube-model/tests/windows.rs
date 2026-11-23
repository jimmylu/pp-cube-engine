//! Rolling windows and time shifts in the model contract.

use pp_cube_model::{load_json, load_yaml, validate_yaml, RollingWindow, TimeShift};

const MODEL: &str = r#"
cubes:
  - name: sales
    sql_table: dw.sales_dtl
    data_source: warehouse
    dimensions:
      - { name: sale_id, sql: "{CUBE}.sale_id", type: number, primary_key: true }
      - { name: channel_cd, sql: "{CUBE}.channel_cd", type: string }
      - { name: sold_at, sql: "{CUBE}.sold_at", type: time }
    measures:
      - { name: amount, type: sum, sql: "{CUBE}.amount" }
      - { name: amount_7d, type: sum, sql: "{CUBE}.amount", rolling_window: { trailing: 7 day, leading: unbounded, offset: start } }
      - { name: amount_mtd, type: sum, sql: "{CUBE}.amount", rolling_window: { type: to_date, granularity: month } }
      - name: amount_prior_year
        type: number
        sql: "{amount}"
        multi_stage: true
        time_shift:
          - { time_dimension: sold_at, interval: 1 year, type: prior }
      - { name: growth, type: number, sql: "{amount} - {amount_prior_year}", multi_stage: true }
"#;

// scenario: B1-MODEL-001
#[test]
fn windows_and_shifts_load_as_written_from_both_formats() {
    let model = load_yaml(MODEL).unwrap();
    let measures = &model.cubes[0].measures;
    assert_eq!(
        measures[1].rolling_window,
        Some(RollingWindow {
            trailing: Some("7 day".into()),
            leading: Some("unbounded".into()),
            offset: Some("start".into()),
            window_type: None,
            granularity: None,
        })
    );
    let to_date = measures[2].rolling_window.as_ref().unwrap();
    assert_eq!(
        (
            to_date.window_type.as_deref(),
            to_date.granularity.as_deref()
        ),
        (Some("to_date"), Some("month"))
    );
    assert_eq!(measures[3].multi_stage, Some(true));
    assert_eq!(
        measures[3].time_shift,
        [TimeShift {
            time_dimension: Some("sold_at".into()),
            interval: "1 year".into(),
            shift_type: "prior".into(),
        }]
    );
    assert!(measures[4].time_shift.is_empty());
    // The same model as JSON is the same model.
    let json = serde_json::to_string(&model).unwrap();
    assert_eq!(load_json(&json).unwrap(), model);
    // A measure without these fields is written without them, so the content of
    // a model that uses none of them is what it was before they existed.
    let plain = serde_json::to_value(&measures[0]).unwrap();
    for field in ["rolling_window", "multi_stage", "time_shift"] {
        assert!(plain.get(field).is_none(), "{field}");
    }
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

fn issue(path: &str, rule: &str) -> Vec<(String, String)> {
    vec![(path.to_owned(), rule.to_owned())]
}

// scenario: B1-VALID-001
#[test]
fn window_rules_are_reported_at_their_fields() {
    let window = "/cubes/0/measures/1/rolling_window";
    let rolling = "rolling_window: { trailing: 7 day, leading: unbounded, offset: start }";
    for (replacement, path, rule) in [
        // Values outside the contract.
        ("rolling_window: { trailing: 7 days }", "/trailing", "type"),
        ("rolling_window: { trailing: 2 hour }", "/trailing", "type"),
        ("rolling_window: { trailing: 0 day }", "/trailing", "type"),
        (
            "rolling_window: { trailing: 1 month 2 day }",
            "/trailing",
            "type",
        ),
        ("rolling_window: { leading: -1 day }", "/leading", "type"),
        (
            "rolling_window: { trailing: 7 day, offset: middle }",
            "/offset",
            "type",
        ),
        (
            "rolling_window: { type: fixed, granularity: month }",
            "/type",
            "type",
        ),
        (
            "rolling_window: { type: to_date, granularity: hour }",
            "/granularity",
            "type",
        ),
        (
            "rolling_window: { trailing: 7 day, origin: x }",
            "/origin",
            "unknown-field",
        ),
        // Fields that do not go together.
        ("rolling_window: { offset: end }", "", "window"),
        ("rolling_window: { type: to_date }", "", "window"),
        (
            "rolling_window: { type: to_date, granularity: month, trailing: 1 day }",
            "",
            "window",
        ),
        (
            "rolling_window: { trailing: 1 day, granularity: month }",
            "",
            "window",
        ),
    ] {
        assert_eq!(
            issues(rolling, replacement),
            issue(&format!("{window}{path}"), rule),
            "{replacement}"
        );
    }
    // A window aggregates rows; a formula has none, and a window is not a later stage.
    assert_eq!(
        issues(
            r#"type: sum, sql: "{CUBE}.amount", rolling_window: { type: to_date, granularity: month }"#,
            r#"type: number, sql: "{amount}", rolling_window: { type: to_date, granularity: month }"#
        ),
        issue("/cubes/0/measures/2/rolling_window", "window")
    );
    assert_eq!(
        issues(
            rolling,
            "rolling_window: { trailing: 7 day }, multi_stage: true"
        ),
        [
            (
                "/cubes/0/measures/1/multi_stage".to_owned(),
                "multi-stage".to_owned()
            ),
            (window.to_owned(), "window".to_owned()),
        ]
    );
}

// scenario: B1-VALID-002
#[test]
fn shift_rules_are_reported_at_their_fields() {
    let shift = "/cubes/0/measures/3/time_shift";
    let written = "- { time_dimension: sold_at, interval: 1 year, type: prior }";
    for (replacement, path, rule) in [
        (
            "- { time_dimension: sold_at, interval: 1 year, type: before }",
            "/0/type",
            "type",
        ),
        (
            "- { time_dimension: sold_at, interval: year, type: prior }",
            "/0/interval",
            "type",
        ),
        (
            "- { time_dimension: sold_at, type: prior }",
            "/0/interval",
            "required",
        ),
        (
            "- { time_dimension: sold_at, interval: 1 year, type: prior, name: x }",
            "/0/name",
            "unknown-field",
        ),
        // Not a time dimension of this cube.
        (
            "- { time_dimension: channel_cd, interval: 1 year, type: prior }",
            "/0/time_dimension",
            "time-shift",
        ),
        (
            "- { time_dimension: missing, interval: 1 year, type: prior }",
            "/0/time_dimension",
            "time-shift",
        ),
        (
            "- { interval: 1 year, type: prior }\n          - { interval: 1 month, type: prior }",
            "",
            "time-shift",
        ),
    ] {
        assert_eq!(
            issues(written, replacement),
            issue(&format!("{shift}{path}"), rule),
            "{replacement}"
        );
    }
    // The time dimension may be left to the query.
    assert!(issues(written, "- { interval: 1 year, type: next }").is_empty());
    // A shift is a later stage of a formula, and only a formula is one.
    assert_eq!(
        issues("multi_stage: true\n        time_shift:", "time_shift:"),
        issue(shift, "multi-stage")
    );
    assert_eq!(
        issues(
            r#"- { name: amount, type: sum, sql: "{CUBE}.amount" }"#,
            r#"- { name: amount, type: sum, sql: "{CUBE}.amount", multi_stage: true }"#
        ),
        issue("/cubes/0/measures/0/multi_stage", "multi-stage")
    );
}
