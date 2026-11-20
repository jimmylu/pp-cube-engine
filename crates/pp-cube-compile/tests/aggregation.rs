//! Single-cube aggregation, compiled for Presto.
//!
//! The snapshots are the SQL the pinned planner emits with the Presto
//! templates. Nobody has executed them; execution belongs to tiers T1 and T2.

use std::sync::Arc;

use pp_cube_compile::{compile, CompileContext, DialectId};
use pp_cube_model::{load_yaml, Model, Query};

fn sales() -> Arc<Model> {
    Arc::new(load_yaml(include_str!("fixtures/sales.yaml")).expect("the model loads"))
}

/// The SQL, then its parameters, as one text to snapshot.
fn compiled(measures: &[&str]) -> String {
    let query = Query {
        measures: measures
            .iter()
            .map(|measure| format!("sales.{measure}"))
            .collect(),
        dimensions: vec!["sales.channel_cd".into()],
    };
    let context = CompileContext {
        dialect: DialectId::Presto,
    };
    let compiled = compile(&sales(), &query, &context).expect("the query compiles");
    format!("{}\n-- parameters: {:?}", compiled.sql, compiled.parameters)
}

#[test]
fn one_sum_grouped_by_one_dimension() {
    insta::assert_snapshot!(compiled(&["amount"]));
}

// scenario: B0-AGG-001
#[test]
fn six_aggregate_types_grouped_by_a_dimension() {
    insta::assert_snapshot!(compiled(&[
        "row_count",
        "order_count",
        "amount",
        "avg_amount",
        "min_amount",
        "max_amount",
    ]));
}

// scenario: B0-AGG-002
#[test]
fn filtered_sum_and_ratio_of_aggregates() {
    insta::assert_snapshot!(compiled(&["toy_amount", "amount_per_order"]));
}
