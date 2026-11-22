//! Single-cube aggregation, compiled for Presto.
//!
//! The snapshots are the SQL the pinned planner emits with the Presto
//! templates. Nobody has executed them; execution belongs to tiers T1 and T2.

mod support;
use support::compile;

use std::sync::Arc;

use pp_cube_compile::{CompileContext, CompileError, Compiled, DialectId};
use pp_cube_model::{load_yaml, Model, Query};

fn model() -> Arc<Model> {
    Arc::new(load_yaml(include_str!("fixtures/sales.yaml")).expect("the model loads"))
}

fn compile_presto(measures: &[&str], dimensions: &[&str]) -> Result<Compiled, CompileError> {
    let query = Query {
        measures: measures.iter().map(|path| (*path).to_owned()).collect(),
        dimensions: dimensions.iter().map(|path| (*path).to_owned()).collect(),
        ..Query::default()
    };
    let context = CompileContext {
        request_id: String::new(),
        dialect: DialectId::Presto,
        timezone: "UTC".to_owned(),
        security_filters: Vec::new(),
    };
    compile(&model(), &query, &context)
}

/// The SQL of `cube`'s measures grouped by its channel, then the parameters,
/// as one text to snapshot.
fn compiled(cube: &str, measures: &[&str]) -> String {
    let measures: Vec<_> = measures
        .iter()
        .map(|name| format!("{cube}.{name}"))
        .collect();
    let measures: Vec<_> = measures.iter().map(String::as_str).collect();
    let compiled =
        compile_presto(&measures, &[&format!("{cube}.channel_cd")]).expect("the query compiles");
    format!("{}\n-- parameters: {:?}", compiled.sql, compiled.parameters)
}

// scenario: B0-DET-001
#[test]
fn one_sum_grouped_by_one_dimension() {
    insta::assert_snapshot!(compiled("sales", &["amount"]));
}

// scenario: B0-AGG-001
// scenario: B0-DET-001
// scenario: B0-PRESTO-004
#[test]
fn six_aggregate_types_grouped_by_a_dimension() {
    insta::assert_snapshot!(compiled(
        "sales",
        &[
            "row_count",
            "order_count",
            "amount",
            "avg_amount",
            "min_amount",
            "max_amount",
        ]
    ));
}

// scenario: B0-AGG-002
// scenario: B0-DET-001
// scenario: B0-PRESTO-005
#[test]
fn filtered_sum_and_ratio_of_aggregates() {
    insta::assert_snapshot!(compiled("sales", &["toy_amount", "amount_per_order"]));
}

// scenario: B0-DET-001
#[test]
fn cube_defined_by_sql_is_selected_as_a_subquery() {
    insta::assert_snapshot!(compiled("valid_sales", &["amount"]));
}

// scenario: B0-DET-001
#[test]
fn boolean_dimension_groups_by_its_condition() {
    let compiled =
        compile_presto(&["sales.amount"], &["sales.is_valid"]).expect("the query compiles");

    insta::assert_snapshot!(compiled.sql);
}

// scenario: B0-DET-001
#[test]
fn count_over_a_cube_with_one_primary_key() {
    insta::assert_snapshot!(compiled("orders", &["row_count"]));
}

// scenario: B0-DET-001
#[test]
fn count_over_a_cube_with_a_composite_primary_key() {
    insta::assert_snapshot!(compiled("order_lines", &["row_count"]));
}

#[test]
fn unknown_member_is_a_query_failure_naming_it() {
    let unknown_measure = compile_presto(&["sales.missing"], &[]);
    let unknown_cube = compile_presto(&[], &["nowhere.channel_cd"]);
    let no_cube = compile_presto(&["amount"], &[]);

    let message = |result: Result<Compiled, CompileError>| match result {
        Err(CompileError::Query(message)) => message,
        other => panic!("expected a query failure, got {other:?}"),
    };
    assert_eq!(
        (
            message(unknown_measure),
            message(unknown_cube),
            message(no_cube)
        ),
        (
            "Measure `sales.missing` is not in the model".to_owned(),
            "Cube `nowhere` is not in the model".to_owned(),
            "`amount` is not written `cube.member`".to_owned(),
        )
    );
}

#[test]
fn internal_failure_does_not_show_its_detail() {
    let error = CompileError::Internal {
        detail: "dialect presto has no template `statements/cte`".to_owned(),
    };

    assert_eq!(error.to_string(), "the query could not be compiled");
}
