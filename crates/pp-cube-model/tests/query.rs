//! The query type, as far as the kernel compiles queries.

use pp_cube_model::{
    load_query, Condition, DateRange, DateValue, Direction, Filter, Granularity, Operator, Order,
    Query, TimeDimension,
};

const FULL: &str = r#"{
  "measures": ["sales.amount"],
  "dimensions": ["sales.channel_cd"],
  "segments": ["sales.valid"],
  "filters": [
    {"member": "sales.channel_cd", "operator": "equals", "values": ["C1", "C2"]},
    {"or": [
      {"member": "sales.qty", "operator": "gt", "values": ["10"]},
      {"and": [
        {"member": "sales.is_valid", "operator": "set"},
        {"member": "sales.amount", "operator": "lte", "values": ["100"]}
      ]}
    ]}
  ],
  "order": {"sales.channel_cd": "asc", "sales.amount": "desc"},
  "limit": 50,
  "offset": 100,
  "timeDimensions": [
    {"dimension": "sales.sold_at", "granularity": "month"},
    {"dimension": "sales.sold_at", "dateRange": ["2026-10-01", "2026-10-31T12:30:00.500"]}
  ],
  "timezone": "Asia/Tokyo"
}"#;

fn condition(member: &str, operator: Operator, values: &[&str]) -> Filter {
    Filter::Condition(Condition {
        member: member.to_owned(),
        operator,
        values: values.iter().map(|value| (*value).to_owned()).collect(),
    })
}

fn order(member: &str, direction: Direction) -> Order {
    Order {
        member: member.to_owned(),
        direction,
    }
}

fn date(text: &str) -> DateValue {
    DateValue::try_from(text.to_owned()).expect("a date value")
}

#[test]
fn query_loads_from_cube_query_json() {
    let expected = Query {
        measures: vec!["sales.amount".into()],
        dimensions: vec!["sales.channel_cd".into()],
        segments: vec!["sales.valid".into()],
        filters: vec![
            condition("sales.channel_cd", Operator::Equals, &["C1", "C2"]),
            Filter::Or(vec![
                condition("sales.qty", Operator::Gt, &["10"]),
                Filter::And(vec![
                    condition("sales.is_valid", Operator::Set, &[]),
                    condition("sales.amount", Operator::Lte, &["100"]),
                ]),
            ]),
        ],
        order: Some(vec![
            order("sales.channel_cd", Direction::Asc),
            order("sales.amount", Direction::Desc),
        ]),
        limit: Some(50),
        offset: Some(100),
        time_dimensions: vec![
            TimeDimension {
                dimension: "sales.sold_at".into(),
                granularity: Some(Granularity::Month),
                date_range: None,
            },
            TimeDimension {
                dimension: "sales.sold_at".into(),
                granularity: None,
                date_range: Some(DateRange {
                    from: date("2026-10-01"),
                    to: date("2026-10-31T12:30:00.500"),
                }),
            },
        ],
        timezone: Some("Asia/Tokyo".into()),
    };

    assert_eq!(load_query(FULL), Ok(expected));
}

#[test]
fn order_is_kept_as_written_in_either_form() {
    let as_object = r#"{"order": {"sales.z": "desc", "sales.a": "asc"}}"#;
    let as_pairs = r#"{"order": [["sales.z", "desc"], ["sales.a", "asc"]]}"#;
    let expected = Some(vec![
        order("sales.z", Direction::Desc),
        order("sales.a", Direction::Asc),
    ]);

    let loaded = [as_object, as_pairs].map(|text| load_query(text).map(|query| query.order));

    assert_eq!(loaded, [Ok(expected.clone()), Ok(expected)]);
}

#[test]
fn order_written_empty_is_not_an_absent_order() {
    let loaded = [r#"{"order": {}}"#, r#"{"order": []}"#, "{}"]
        .map(|text| load_query(text).map(|query| query.order));

    assert_eq!(loaded, [Ok(Some(vec![])), Ok(Some(vec![])), Ok(None)]);
}

#[test]
fn empty_and_absent_order_each_survive_a_json_round_trip() {
    let round_trip = |text: &str| {
        let query = load_query(text).expect("the query loads");
        let written = serde_json::to_string(&query).expect("a query serialises");
        load_query(&written).map(|query| query.order)
    };

    assert_eq!(
        [round_trip(r#"{"order": []}"#), round_trip("{}")],
        [Ok(Some(vec![])), Ok(None)]
    );
}

#[test]
fn query_survives_a_json_round_trip() {
    let query = load_query(FULL).expect("the query loads");

    let text = serde_json::to_string(&query).expect("a query serialises");

    assert_eq!(load_query(&text), Ok(query));
}

#[test]
fn filter_value_that_is_not_a_string_is_rejected() {
    let text = r#"{"filters": [{"member": "sales.qty", "operator": "gt", "values": [10]}]}"#;

    let error = load_query(text).expect_err("values are strings in Cube's format");

    assert!(
        error.message.contains("expected a string"),
        "{}",
        error.message
    );
}

#[test]
fn operator_outside_the_value_operators_is_rejected_where_it_is_written() {
    for operator in ["in", "notIn", "onTheDate", "like"] {
        let text = format!(
            "{{\"filters\": [\n  {{\"member\": \"sales.qty\", \"operator\": \"{operator}\"}}\n]}}"
        );

        let error = load_query(&text).expect_err("the operator is not compiled");

        assert_eq!(
            (
                error.position.map(|at| at.line),
                error.message.contains(operator)
            ),
            (Some(2), true),
            "{operator}: {}",
            error.message
        );
    }
}

#[test]
fn filter_that_is_neither_a_condition_nor_one_group_is_rejected() {
    let texts = [
        r#"{"filters": [{"member": "sales.qty", "operator": "set", "or": []}]}"#,
        r#"{"filters": [{"and": [], "or": []}]}"#,
        r#"{"filters": [{"member": "sales.qty"}]}"#,
        r#"{"filters": [{}]}"#,
    ];

    for text in texts {
        let error = load_query(text).expect_err("the filter has no single meaning");

        assert!(
            error.message.contains("a filter is either"),
            "{text}: {}",
            error.message
        );
    }
}

#[test]
fn query_field_the_kernel_does_not_compile_is_rejected() {
    let text = r#"{"measures": ["sales.amount"], "ungrouped": true}"#;

    let error = load_query(text).expect_err("detail queries are outside the baseline");

    assert!(error.message.contains("ungrouped"), "{}", error.message);
}

#[test]
fn granularity_outside_the_five_is_rejected_where_it_is_written() {
    for granularity in ["hour", "fortnight"] {
        let text = format!(
            "{{\"timeDimensions\": [\n  {{\"dimension\": \"sales.sold_at\", \"granularity\": \"{granularity}\"}}\n]}}"
        );

        let error = load_query(&text).expect_err("the granularity is outside the baseline");

        assert_eq!(
            (
                error.position.map(|at| at.line),
                error.message.contains(granularity),
                error.message.contains("`quarter`"),
            ),
            (Some(2), true, true),
            "{granularity}: {}",
            error.message
        );
    }
}

#[test]
fn date_range_that_is_not_two_dates_is_rejected() {
    let failure = |range: &str| {
        let text =
            format!(r#"{{"timeDimensions": [{{"dimension": "s.at", "dateRange": {range}}}]}}"#);
        load_query(&text)
            .expect_err("the range is not two dates")
            .message
    };
    let two_dates = "a list of two dates, the start and the end of the range";

    for range in [r#""last month""#, r#""2026-10-01""#, r#"["2026-10-01"]"#] {
        let message = failure(range);
        assert!(message.contains(two_dates), "{range}: {message}");
    }
    let three = failure(r#"["2026-10-01", "2026-10-02", "2026-10-03"]"#);
    assert!(three.contains(two_dates), "{three}");
    let relative = failure(r#"["2026-10-01", "today"]"#);
    assert!(relative.contains("`today` is not a date"), "{relative}");
}

#[test]
fn date_value_is_a_real_day_and_time_without_a_zone() {
    let accepted = |text: &str| DateValue::try_from(text.to_owned()).is_ok();

    let valid = [
        "2026-10-01",
        "2024-02-29",
        "2026-10-01T00:00:00",
        "2026-10-31T23:59:59.999",
    ];
    let invalid = [
        "2026-02-29",
        "2026-13-01",
        "2026-04-31",
        "2026-10-00",
        "2026-10-01T24:00:00",
        "2026-10-01T10:60:00",
        "2026-10-01 10:00:00",
        "2026-10-01T10:00:00Z",
        "2026-10-01T10:00:00.5",
        "2026-10-01T10:00:00+09:00",
        "2026-1-01",
        "last month",
        "",
    ];

    assert_eq!(
        (valid.map(accepted), invalid.map(accepted)),
        ([true; 4], [false; 13])
    );
}
