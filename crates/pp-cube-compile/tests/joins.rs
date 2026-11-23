//! Joins to dimension cubes, compiled for Presto.
//!
//! The snapshots are the SQL the pinned planner emits with the Presto
//! templates, followed by the parameters in placeholder order. Nobody has
//! executed them; execution belongs to tiers T1 and T2.

mod support;
use support::compile;

use std::sync::Arc;

use pp_cube_compile::{CompileContext, CompileError, Compiled, DialectId};
use pp_cube_model::{load_query, load_yaml};

fn compile_with(model: &str, query: &str) -> Result<Compiled, CompileError> {
    let model = Arc::new(load_yaml(model).expect("the model loads"));
    let query = load_query(query).expect("the query loads");
    let context = CompileContext {
        request_id: String::new(),
        dialect: DialectId::Presto,
        timezone: "UTC".to_owned(),
        security_filters: Vec::new(),
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
// scenario: B0-DET-001
// scenario: B0-PRESTO-006
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

// scenario: B0-DET-001
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
    let two_roots = compile_with(
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
    ).unwrap_err();

    assert_eq!(
        two_paths,
        "There is more than one join path from `refunds` to `regions`"
    );
    assert!(
        matches!(two_roots, CompileError::Model(issues) if issues.iter().any(|i| i.rule_id == "duplicate-join"))
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
    dimensions: [{ name: id, sql: "{CUBE}.id", type: number, primary_key: true }]
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

// scenario: B0-JOIN-006
// scenario: B0-DET-001
#[test]
fn measures_of_a_joined_cube_that_stay_in_one_select() {
    // Through one_to_one every measure type stays flat; through many_to_one the
    // counts do, as distinct counts of the joined cube's keys.
    insta::assert_snapshot!([
        r#"{"measures": ["receipts.tax", "receipts.avg_tax", "receipts.receipt_count", "receipts.tax_per_receipt"],
            "dimensions": ["sales.channel_cd"], "order": [["sales.channel_cd", "asc"]]}"#,
        r#"{"measures": ["stores.store_count", "stores.name_count"],
            "dimensions": ["sales.channel_cd"], "order": [["sales.channel_cd", "asc"]]}"#,
    ]
    .map(compiled)
    .join("\n\n"));
}

// scenario: B0-JOIN-006
// scenario: B1-MULTI-004
#[test]
fn cube_reached_only_through_a_member_expression_counts_as_joined() {
    // Every member below is one of `receipts`; two of them read `sales`,
    // which is the cube the join starts from.
    let model = r#"
cubes:
  - name: sales
    sql_table: dw.sales_dtl
    data_source: warehouse
    joins: [{ name: receipts, relationship: one_to_one, sql: "{CUBE}.receipt_no = {receipts.receipt_no}" }]
    dimensions: [{ name: channel_cd, sql: "{CUBE}.channel_cd", type: string, primary_key: true }]
    measures: [{ name: amount, type: sum, sql: "{CUBE}.amount" }]
  - name: receipts
    sql_table: dw.receipt
    data_source: warehouse
    dimensions:
      - { name: receipt_no, sql: "{CUBE}.receipt_no", type: string, primary_key: true }
      - { name: channel, sql: "{sales.channel_cd}", type: string }
    measures:
      - { name: tax, type: sum, sql: "{CUBE}.tax" }
      - { name: tax_share, type: number, sql: "{tax} / NULLIF({sales.amount}, 0)" }
"#;

    // The dimension brings `sales` in, the join starts there, and the sum stays flat.
    let through_a_dimension = compile_with(
        model,
        r#"{"measures": ["receipts.tax"], "dimensions": ["receipts.channel"]}"#,
    )
    .expect("a one_to_one measure compiles");
    assert!(
        through_a_dimension
            .sql
            .contains(r#"FROM  dw.sales_dtl  AS "sales""#)
            && through_a_dimension.sql.contains("LEFT JOIN  dw.receipt")
            && through_a_dimension.sql.contains(r#"sum("receipts".tax)"#)
            && !through_a_dimension.sql.contains("WITH"),
        "{}",
        through_a_dimension.sql
    );
    // A formula over measures of both cubes aggregates each in the same SELECT.
    let formula = compile_with(model, r#"{"measures": ["receipts.tax_share"]}"#)
        .expect("a formula over two cubes compiles");
    assert!(
        formula
            .sql
            .contains(r#"sum("receipts".tax) / NULLIF(sum("sales".amount), 0)"#)
            && !formula.sql.contains("WITH"),
        "{}",
        formula.sql
    );
}

// scenario: B0-JOIN-006
#[test]
fn cube_the_planner_joins_on_its_own_account_starts_the_tree() {
    // A count has no expression; the planner counts the primary key, and
    // this one reads `sales`. The query names only `receipts`.
    let compiled = compile_with(
        r#"
cubes:
  - name: sales
    sql_table: dw.sales_dtl
    data_source: warehouse
    joins: [{ name: receipts, relationship: one_to_one, sql: "{CUBE}.receipt_no = {receipts.receipt_no}" }]
    dimensions: [{ name: receipt_no, sql: "{CUBE}.receipt_no", type: string, primary_key: true }]
  - name: receipts
    sql_table: dw.receipt
    data_source: warehouse
    dimensions: [{ name: receipt_no, sql: "{sales.receipt_no}", type: string, primary_key: true }]
    measures: [{ name: row_count, type: count }]
"#,
        r#"{"measures": ["receipts.row_count"]}"#,
    )
    .expect("a count through a one_to_one join compiles");

    // The tree starts from the cube that reaches the other, and both are reported.
    assert!(
        compiled.sql.contains(r#"FROM  dw.sales_dtl  AS "sales""#)
            && compiled.sql.contains("LEFT JOIN  dw.receipt")
            && !compiled.sql.contains("WITH"),
        "{}",
        compiled.sql
    );
    let used: Vec<_> = compiled
        .used_cubes
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert_eq!(used, ["receipts", "sales"]);
}

// scenario: B0-JOIN-007
// scenario: B0-DET-001
#[test]
fn cubes_a_join_condition_reads_are_joined_before_it() {
    // The join to `promos` reads `regions`, which is reached through `stores`:
    // both are joined first, whether or not the query names them, and in that
    // order even when the query names `regions` last.
    insta::assert_snapshot!([
        r#"{"measures": ["sales.amount"], "dimensions": ["promos.promo_name"],
            "order": [["promos.promo_name", "asc"]]}"#,
        r#"{"measures": ["sales.amount"], "dimensions": ["promos.promo_name", "regions.region_name"],
            "filters": [{"member": "promos.promo_cd", "operator": "equals", "values": ["P1"]}],
            "order": [["promos.promo_name", "asc"]]}"#,
    ]
    .map(compiled)
    .join("\n\n"));
    // A query that does not need that join is not given its cubes.
    let alone = compile_with(
        include_str!("fixtures/retail.yaml"),
        r#"{"measures": ["sales.amount"], "dimensions": ["sales.channel_cd"]}"#,
    )
    .expect("a query on sales alone compiles");
    assert!(!alone.sql.contains("JOIN"), "{}", alone.sql);
}

// scenario: B0-JOIN-005
#[test]
fn join_conditions_that_cannot_be_ordered_are_refused_where_they_are_needed() {
    // Here the join to `stores` reads `regions`, and `regions` is reached only
    // through `stores`: neither can be joined first.
    let model = |condition: &str| {
        r#"
cubes:
  - name: sales
    sql_table: dw.sales_dtl
    data_source: warehouse
    joins: [{ name: stores, relationship: many_to_one, sql: "CONDITION" }]
    dimensions: [{ name: sale_id, sql: "{CUBE}.sale_id", type: number, primary_key: true }]
    measures: [{ name: amount, type: sum, sql: "{CUBE}.amount" }]
  - name: stores
    sql_table: dw.dim_store
    data_source: warehouse
    joins: [{ name: regions, relationship: many_to_one, sql: "{CUBE}.region_cd = {regions.region_cd}" }]
    dimensions:
      - { name: store_cd, sql: "{CUBE}.store_cd", type: string, primary_key: true }
      - { name: store_name, sql: "{CUBE}.store_name", type: string }
      - { name: region_key, sql: "{regions.region_cd}", type: string }
  - name: regions
    sql_table: dw.dim_region
    data_source: warehouse
    dimensions:
      - { name: region_cd, sql: "{CUBE}.region_cd", type: string, primary_key: true }
      - { name: active, sql: "{CUBE}.active", type: number }
"#
        .replace("CONDITION", condition)
    };
    let through_a_dimension = model("{CUBE}.region_cd = {stores.region_key}");
    let directly = model("{CUBE}.store_cd = {stores.store_cd} AND {regions.active} = 1");
    let by_store = r#"{"measures": ["sales.amount"], "dimensions": ["stores.store_name"]}"#;
    let refused = "The joins to `stores`, `regions` cannot be ordered: a join condition reads a \
                   cube that is joined only after it";

    assert_eq!(
        [
            failure_with(&through_a_dimension, by_store),
            failure_with(&directly, by_store),
        ],
        [refused, refused]
    );
    let alone = compile_with(&directly, r#"{"measures": ["sales.amount"]}"#)
        .expect("a query that does not need the join compiles");
    assert!(!alone.sql.contains("JOIN"), "{}", alone.sql);
}

// scenario: B0-JOIN-002
#[test]
fn deep_model_with_two_joins_per_cube_is_answered_without_walking_every_chain() {
    // Thirty layers of two cubes, each cube joining both cubes of the next
    // layer: hundreds of millions of chains lead from the first layer to the
    // last, and as many lead nowhere near a cube that is not connected.
    const LAYERS: usize = 30;
    let mut model = String::from("cubes:\n");
    let mut cube = |name: &str, joined: &[String]| {
        model.push_str(&format!(
            "  - name: {name}\n    sql_table: dw.{name}\n    data_source: warehouse\n"
        ));
        if !joined.is_empty() {
            model.push_str("    joins:\n");
        }
        for other in joined {
            model.push_str(&format!(
                "      - {{ name: {other}, relationship: many_to_one, sql: \"{{CUBE}}.{other}_id = {{{other}.id}}\" }}\n"
            ));
        }
        model.push_str(
            "    dimensions:\n      - { name: id, sql: \"{CUBE}.id\", type: number, primary_key: true }\n",
        );
    };
    for layer in 0..LAYERS {
        let next: Vec<String> = ["a", "b"]
            .iter()
            .filter(|_| layer + 1 < LAYERS)
            .map(|side| format!("l{}{side}", layer + 1))
            .collect();
        cube(&format!("l{layer}a"), &next);
        cube(&format!("l{layer}b"), &next);
    }
    cube("island", &[]);

    let unconnected = failure_with(&model, r#"{"dimensions": ["l0a.id", "island.id"]}"#);
    let ambiguous = failure_with(&model, r#"{"dimensions": ["l0a.id", "l29a.id"]}"#);

    assert_eq!(
        (unconnected, ambiguous),
        (
            "No join path connects cubes `l0a`, `island`: none of them reaches all the others"
                .to_owned(),
            "There is more than one join path from `l0a` to `l29a`".to_owned(),
        )
    );
}

#[test]
fn join_to_a_cube_the_model_lacks_names_both_cubes() {
    let error = compile_with(
        r#"
cubes:
  - name: orders
    sql_table: dw.order_hdr
    data_source: warehouse
    joins: [{ name: customers, relationship: many_to_one, sql: "{CUBE}.customer_id = {customers.customer_id}" }]
    measures: [{ name: row_count, type: count }]
"#,
        r#"{"measures": ["orders.row_count"]}"#,
    ).unwrap_err();

    assert!(
        matches!(error, CompileError::Model(issues) if issues.iter().any(|i| i.rule_id == "join-target"))
    );
}
