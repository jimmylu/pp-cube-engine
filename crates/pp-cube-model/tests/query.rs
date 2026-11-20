//! The query type, as far as the kernel compiles queries.

use pp_cube_model::{load_query, Condition, Direction, Filter, Operator, Order, Query};

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
  "offset": 100
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
    for operator in ["in", "notIn", "inDateRange", "beforeDate", "like"] {
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
fn query_field_the_kernel_does_not_compile_yet_is_rejected() {
    let text = r#"{"measures": ["sales.amount"], "timeDimensions": []}"#;

    let error = load_query(text).expect_err("time dimensions are not compiled yet");

    assert!(
        error.message.contains("timeDimensions"),
        "{}",
        error.message
    );
}
