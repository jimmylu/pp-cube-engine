//! Joins to dimension cubes, compiled for Presto.
//!
//! The snapshots are the SQL the pinned planner emits with the Presto
//! templates, followed by the parameters in placeholder order. Nobody has
//! executed them; execution belongs to tiers T1 and T2.

use std::sync::Arc;

use pp_cube_compile::{compile, CompileContext, CompileError, Compiled, DialectId};
use pp_cube_model::{load_query, load_yaml};

fn compile_with(model: &str, query: &str) -> Result<Compiled, CompileError> {
    let model = Arc::new(load_yaml(model).expect("the model loads"));
    let query = load_query(query).expect("the query loads");
    let context = CompileContext {
        dialect: DialectId::Presto,
        timezone: "UTC".to_owned(),
    };
    compile(&model, &query, &context)
}

/// SQL and parameters of `query` over the retail model, as one text to snapshot.
fn compiled(query: &str) -> String {
    let compiled =
        compile_with(include_str!("fixtures/retail.yaml"), query).expect("the query compiles");
    format!("{}\n-- parameters: {:?}", compiled.sql, compiled.parameters)
}

fn failure_with(model: &str, query: &str) -> String {
    match compile_with(model, query) {
        Err(CompileError::Query(message)) => message,
        other => panic!("expected a query failure, got {other:?}"),
    }
}

fn query_failure(query: &str) -> String {
    failure_with(include_str!("fixtures/retail.yaml"), query)
}

// scenario: B0-JOIN-001
#[test]
fn sales_grouped_and_filtered_by_a_store_dimension() {
    insta::assert_snapshot!(compiled(
        r#"{
          "measures": ["sales.amount"],
          "dimensions": ["stores.store_name"],
          "filters": [{"member": "stores.store_name", "operator": "startsWith", "values": ["North"]}]
        }"#
    ));
}

#[test]
fn dimension_two_joins_away_is_reached_through_the_cube_between() {
    insta::assert_snapshot!(compiled(
        r#"{
          "measures": ["sales.amount"],
          "dimensions": ["regions.region_name", "receipts.printed"],
          "segments": ["stores.open"],
          "timeDimensions": [{"dimension": "sales.sold_at", "granularity": "month"}]
        }"#
    ));
}

// scenario: B0-JOIN-002
#[test]
fn second_path_to_a_cube_is_refused_instead_of_choosing_one() {
    let two_paths =
        query_failure(r#"{"measures": ["refunds.amount"], "dimensions": ["regions.region_name"]}"#);
    let two_roots = failure_with(
        r#"
cubes:
  - name: orders
    sql_table: dw.order_hdr
    data_source: warehouse
    joins: [{ name: invoices, relationship: one_to_one, sql: "{CUBE}.order_no = {invoices.order_no}" }]
    dimensions: [{ name: order_no, sql: "{CUBE}.order_no", type: number, primary_key: true }]
  - name: invoices
    sql_table: dw.invoice
    data_source: warehouse
    joins: [{ name: orders, relationship: one_to_one, sql: "{CUBE}.order_no = {orders.order_no}" }]
    dimensions: [{ name: order_no, sql: "{CUBE}.order_no", type: number, primary_key: true }]
"#,
        r#"{"dimensions": ["orders.order_no", "invoices.order_no"]}"#,
    );

    assert_eq!(
        (two_paths, two_roots),
        (
            "There is more than one join path from `refunds` to `regions`".to_owned(),
            "Cubes `orders`, `invoices` can be joined starting from `orders` or from \
             `invoices`; there is more than one join path"
                .to_owned(),
        )
    );
}

// scenario: B0-JOIN-003
#[test]
fn cubes_on_different_data_sources_are_not_joined() {
    let direct =
        query_failure(r#"{"measures": ["sales.amount"], "dimensions": ["visits.source"]}"#);
    // `orders` and `countries` share a data source; the cube between them does not.
    let passed_through = failure_with(
        r#"
cubes:
  - name: orders
    sql_table: dw.order_hdr
    data_source: warehouse
    joins: [{ name: customers, relationship: many_to_one, sql: "{CUBE}.customer_id = {customers.customer_id}" }]
    measures: [{ name: row_count, type: count }]
  - name: customers
    sql_table: crm.customer
    data_source: crm
    joins: [{ name: countries, relationship: many_to_one, sql: "{CUBE}.country_cd = {countries.country_cd}" }]
    dimensions: [{ name: customer_id, sql: "{CUBE}.customer_id", type: number, primary_key: true }]
  - name: countries
    sql_table: dw.dim_country
    data_source: warehouse
    dimensions: [{ name: country_cd, sql: "{CUBE}.country_cd", type: string, primary_key: true }]
"#,
        r#"{"measures": ["orders.row_count"], "dimensions": ["countries.country_cd"]}"#,
    );

    assert_eq!(
        (direct, passed_through),
        (
            "Cubes on different data sources cannot be joined: `sales` on `warehouse`, `visits` \
             on `lake`"
                .to_owned(),
            "Cubes on different data sources cannot be joined: `orders` on `warehouse`, \
             `customers` on `crm`"
                .to_owned(),
        )
    );
}

// scenario: B0-JOIN-004
#[test]
fn cubes_that_no_join_path_connects_are_refused_by_name() {
    let unconnected = query_failure(
        r#"{"dimensions": ["sales.channel_cd", "refunds.refund_id", "stores.store_name"]}"#,
    );
    let against_the_join = query_failure(
        r#"{"dimensions": ["regions.region_name"], "segments": ["stores.open"],
            "filters": [{"member": "receipts.printed", "operator": "set"}]}"#,
    );

    assert_eq!(
        (unconnected, against_the_join),
        (
            "No join path connects cubes `sales`, `refunds`, `stores`: none of them reaches all \
             the others"
                .to_owned(),
            "No join path connects cubes `regions`, `stores`, `receipts`: none of them reaches \
             all the others"
                .to_owned(),
        )
    );
}

// scenario: B0-JOIN-004
#[test]
fn measure_of_a_joined_cube_is_refused_under_either_relationship() {
    let many_to_one = query_failure(
        r#"{"measures": ["stores.store_count"], "dimensions": ["sales.channel_cd"]}"#,
    );
    let one_to_one =
        query_failure(r#"{"measures": ["receipts.tax"], "dimensions": ["sales.channel_cd"]}"#);
    let in_a_filter = query_failure(
        r#"{"measures": ["sales.amount"],
            "filters": [{"member": "stores.store_count", "operator": "gt", "values": ["1"]}]}"#,
    );

    assert_eq!(
        [many_to_one, one_to_one, in_a_filter],
        [
            "Measure `stores.store_count` belongs to `stores`, which this query reaches through \
             a join from `sales`; measures are taken from the cube the joins start from",
            "Measure `receipts.tax` belongs to `receipts`, which this query reaches through a \
             join from `sales`; measures are taken from the cube the joins start from",
            "Measures of more than one cube (`sales`, `stores`) are outside the baseline",
        ]
    );
}

#[test]
fn join_to_a_cube_the_model_lacks_names_both_cubes() {
    let message = failure_with(
        r#"
cubes:
  - name: orders
    sql_table: dw.order_hdr
    data_source: warehouse
    joins: [{ name: customers, relationship: many_to_one, sql: "{CUBE}.customer_id = {customers.customer_id}" }]
    measures: [{ name: row_count, type: count }]
"#,
        r#"{"measures": ["orders.row_count"]}"#,
    );

    assert_eq!(
        message,
        "Cube `orders` joins `customers`, which is not in the model"
    );
}
