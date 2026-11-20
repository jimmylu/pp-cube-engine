//! Single-cube aggregation, compiled for Presto.
//!
//! The snapshots are the SQL the pinned planner emits with the Presto
//! templates. Nobody has executed them; execution belongs to tiers T1 and T2.

use std::sync::Arc;

use pp_cube_compile::{compile, CompileContext, CompileError, Compiled, DialectId};
use pp_cube_model::{load_yaml, Model, Query};

fn model() -> Arc<Model> {
    Arc::new(load_yaml(include_str!("fixtures/sales.yaml")).expect("the model loads"))
}

fn compile_presto(measures: &[&str], dimensions: &[&str]) -> Result<Compiled, CompileError> {
    let query = Query {
        measures: measures.iter().map(|path| (*path).to_owned()).collect(),
        dimensions: dimensions.iter().map(|path| (*path).to_owned()).collect(),
    };
    let context = CompileContext {
        dialect: DialectId::Presto,
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

#[test]
fn one_sum_grouped_by_one_dimension() {
    insta::assert_snapshot!(compiled("sales", &["amount"]));
}

// scenario: B0-AGG-001
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
#[test]
fn filtered_sum_and_ratio_of_aggregates() {
    insta::assert_snapshot!(compiled("sales", &["toy_amount", "amount_per_order"]));
}

#[test]
fn cube_defined_by_sql_is_selected_as_a_subquery() {
    insta::assert_snapshot!(compiled("valid_sales", &["amount"]));
}

#[test]
fn boolean_dimension_groups_by_its_condition() {
    let compiled =
        compile_presto(&["sales.amount"], &["sales.is_valid"]).expect("the query compiles");

    insta::assert_snapshot!(compiled.sql);
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
fn what_the_kernel_does_not_compile_yet_fails_without_showing_internals() {
    let two_cubes = compile_presto(&["sales.amount", "valid_sales.amount"], &[]);

    let error = two_cubes.expect_err("a query over two cubes is not compiled yet");

    assert!(matches!(error, CompileError::Internal { .. }), "{error:?}");
    assert_eq!(error.to_string(), "the query could not be compiled");
}
