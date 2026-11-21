//! Filters, segments, order and row limits, compiled for Presto.
//!
//! The snapshots are the SQL the pinned planner emits with the Presto
//! templates, followed by the parameters in placeholder order. Nobody has
//! executed them; execution belongs to tiers T1 and T2.

mod support;
use support::compile;

use std::sync::Arc;

use pp_cube_compile::{CompileContext, CompileError, Compiled, DialectId};
use pp_cube_model::{load_query, load_yaml};

fn compile_presto(query: &str) -> Result<Compiled, CompileError> {
    let model = Arc::new(load_yaml(include_str!("fixtures/sales.yaml")).expect("the model loads"));
    let query = load_query(query).expect("the query loads");
    let context = CompileContext {
        request_id: String::new(),
        dialect: DialectId::Presto,
        timezone: "UTC".to_owned(),
        security_filters: Vec::new(),
    };
    compile(&model, &query, &context)
}

/// `sales.amount` by channel with `rest` added to the query, as SQL and parameters.
fn compiled(rest: &str) -> String {
    let query =
        format!(r#"{{"measures": ["sales.amount"], "dimensions": ["sales.channel_cd"], {rest}}}"#);
    let compiled = compile_presto(&query).expect("the query compiles");
    format!("{}\n-- parameters: {:?}", compiled.sql, compiled.parameters)
}

fn query_failure(rest: &str) -> String {
    let query = format!(r#"{{"measures": ["sales.amount"], {rest}}}"#);
    match compile_presto(&query) {
        Err(CompileError::Query(message)) => message,
        other => panic!("expected a query failure, got {other:?}"),
    }
}

#[test]
fn equals_on_a_string_dimension() {
    insta::assert_snapshot!(compiled(
        r#""filters": [{"member": "sales.channel_cd", "operator": "equals", "values": ["C1"]}]"#
    ));
}

// scenario: B0-QUERY-006
#[test]
fn equality_and_presence_operators() {
    insta::assert_snapshot!(compiled(
        r#""filters": [
            {"member": "sales.channel_cd", "operator": "equals", "values": ["C1", "C2"]},
            {"member": "sales.channel_cd", "operator": "notEquals", "values": ["C9"]},
            {"member": "sales.is_valid", "operator": "equals", "values": ["true"]},
            {"member": "sales.qty", "operator": "set"},
            {"member": "sales.sold_at", "operator": "notSet"}
        ]"#
    ));
}

// scenario: B0-QUERY-006
#[test]
fn string_operators_escape_the_pattern_characters_of_their_values() {
    insta::assert_snapshot!(compiled(
        r#""filters": [
            {"member": "sales.channel_cd", "operator": "contains", "values": ["50%_off"]},
            {"member": "sales.channel_cd", "operator": "notContains", "values": ["x"]},
            {"member": "sales.channel_cd", "operator": "startsWith", "values": ["C"]},
            {"member": "sales.channel_cd", "operator": "notStartsWith", "values": ["Z"]},
            {"member": "sales.channel_cd", "operator": "endsWith", "values": ["1"]},
            {"member": "sales.channel_cd", "operator": "notEndsWith", "values": ["9"]}
        ]"#
    ));
}

// scenario: B0-QUERY-006
#[test]
fn comparison_operators_on_a_number_dimension() {
    insta::assert_snapshot!(compiled(
        r#""filters": [
            {"member": "sales.qty", "operator": "gt", "values": ["1"]},
            {"member": "sales.qty", "operator": "gte", "values": ["2"]},
            {"member": "sales.qty", "operator": "lt", "values": ["30"]},
            {"member": "sales.qty", "operator": "lte", "values": ["40"]}
        ]"#
    ));
}

// scenario: B0-QUERY-006
#[test]
fn filters_on_measures_restrict_groups() {
    insta::assert_snapshot!(compiled(
        r#""filters": [
            {"member": "sales.amount", "operator": "gt", "values": ["1000"]},
            {"member": "sales.order_count", "operator": "notEquals", "values": ["0"]},
            {"member": "sales.channel_cd", "operator": "equals", "values": ["C1"]}
        ]"#
    ));
}

// scenario: B0-QUERY-005
#[test]
fn top_level_filters_are_joined_with_and_and_groups_keep_their_nesting() {
    insta::assert_snapshot!(compiled(
        r#""filters": [
            {"member": "sales.is_valid", "operator": "equals", "values": ["true"]},
            {"or": [
                {"member": "sales.channel_cd", "operator": "equals", "values": ["C1"]},
                {"and": [
                    {"member": "sales.qty", "operator": "gt", "values": ["10"]},
                    {"member": "sales.channel_cd", "operator": "startsWith", "values": ["D"]}
                ]}
            ]}
        ]"#
    ));
}

// scenario: B0-QUERY-007
#[test]
fn segment_order_limit_and_offset() {
    insta::assert_snapshot!(compiled(
        r#""segments": ["sales.toys"],
           "order": {"sales.channel_cd": "asc", "sales.amount": "desc"},
           "limit": 50,
           "offset": 100"#
    ));
}

#[test]
fn order_written_empty_turns_the_default_order_off() {
    let ordered = |rest: &str| compiled(rest).contains("ORDER BY");

    assert_eq!(
        [
            ordered(r#""order": {}"#),
            ordered(r#""order": []"#),
            ordered(r#""limit": 5"#),
        ],
        [false, false, true]
    );
}

// scenario: B0-QUERY-002
#[test]
fn filter_on_an_unknown_member_names_it() {
    let unknown = query_failure(
        r#""filters": [{"member": "sales.missing", "operator": "equals", "values": ["1"]}]"#,
    );
    let nested =
        query_failure(r#""filters": [{"or": [{"member": "nowhere.qty", "operator": "set"}]}]"#);

    assert_eq!(
        (unknown, nested),
        (
            "`sales.missing` is not a dimension or a measure of the model".to_owned(),
            "Cube `nowhere` is not in the model".to_owned(),
        )
    );
}

// scenario: B0-QUERY-002
#[test]
fn operator_that_does_not_fit_its_member_names_the_rule() {
    let failure = |member: &str, operator: &str| {
        query_failure(&format!(
            r#""filters": [{{"member": "sales.{member}", "operator": "{operator}", "values": ["1"]}}]"#
        ))
    };

    assert_eq!(
        [
            failure("qty", "contains"),
            failure("channel_cd", "gt"),
            failure("is_valid", "lte"),
            failure("sold_at", "startsWith"),
            failure("sold_at", "equals"),
            failure("amount", "endsWith"),
        ],
        [
            "Operator `contains` does not fit `sales.qty`, a number dimension",
            "Operator `gt` does not fit `sales.channel_cd`, a string dimension",
            "Operator `lte` does not fit `sales.is_valid`, a boolean dimension",
            "Operator `startsWith` does not fit `sales.sold_at`, a time dimension",
            "Operator `equals` does not fit `sales.sold_at`, a time dimension",
            "Operator `endsWith` does not fit `sales.amount`, a measure",
        ]
    );
}

// scenario: B0-QUERY-002
#[test]
fn operator_outside_the_baseline_is_refused_when_the_query_is_read() {
    let query = r#"{"filters": [{"member": "sales.qty", "operator": "in", "values": ["1"]}]}"#;

    let error = load_query(query).expect_err("`in` is not a baseline operator");

    assert!(
        error.message.contains("unknown variant `in`"),
        "{}",
        error.message
    );
}

#[test]
fn values_must_match_what_the_operator_takes() {
    let none = query_failure(r#""filters": [{"member": "sales.qty", "operator": "gt"}]"#);
    let some = query_failure(
        r#""filters": [{"member": "sales.qty", "operator": "set", "values": ["1"]}]"#,
    );

    assert_eq!(
        (none, some),
        (
            "Operator `gt` on `sales.qty` needs at least one value".to_owned(),
            "Operator `set` on `sales.qty` takes no values".to_owned(),
        )
    );
}

#[test]
fn unknown_segment_is_named() {
    let segment = query_failure(r#""segments": ["sales.missing"]"#);

    assert_eq!(segment, "Segment `sales.missing` is not in the model");
}

#[test]
fn order_by_a_member_the_query_does_not_select_is_refused() {
    let order =
        query_failure(r#""dimensions": ["sales.channel_cd"], "order": {"sales.qty": "asc"}"#);

    assert_eq!(
        order,
        "Order member `sales.qty` is not among the query's measures and dimensions"
    );
}
