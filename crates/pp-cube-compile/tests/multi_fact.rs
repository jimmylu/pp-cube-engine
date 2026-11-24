//! Measures of more than one cube in one query, compiled for Presto and Doris.
//!
//! The snapshots are the SQL the pinned planner emits with each dialect's
//! templates, followed by the parameters in placeholder order. The Presto text
//! of every query here was compared with Cube Core at the release of the pin;
//! execution belongs to the engine tiers.

mod support;
use support::compile;

use std::sync::Arc;

use pp_cube_compile::{CompileContext, CompileError, Compiled, DialectId, Parameter};
use pp_cube_model::{load_query, load_yaml};

const RETAIL: &str = include_str!("fixtures/retail.yaml");

fn context(dialect: DialectId) -> CompileContext {
    CompileContext {
        request_id: String::new(),
        dialect,
        timezone: "UTC".to_owned(),
        security_filters: Vec::new(),
    }
}

fn compile_in(context: &CompileContext, query: &str) -> Result<Compiled, CompileError> {
    let model = Arc::new(load_yaml(RETAIL).expect("the model loads"));
    let query = load_query(query).expect("the query loads");
    compile(&model, &query, context)
}

fn presto(query: &str) -> Compiled {
    compile_in(&context(DialectId::Presto), query).expect("the query compiles")
}

/// SQL and parameters of `query` in `dialect`, as one text to snapshot.
fn compiled(dialect: DialectId, query: &str) -> String {
    let compiled = compile_in(&context(dialect), query).expect("the query compiles");
    format!("{}\n-- parameters: {:?}", compiled.sql, compiled.parameters)
}

fn text(values: &[&str]) -> Vec<Parameter> {
    let values = values.iter();
    values
        .map(|value| Parameter::Text((*value).into()))
        .collect()
}

const BY_STORE: &str = r#"{"measures": ["sales.amount", "refunds.amount"],
    "dimensions": ["stores.store_name"], "order": [["stores.store_name", "asc"]]}"#;
const TOTALS: &str = r#"{"measures": ["sales.amount", "refunds.amount"]}"#;
const FILTERED_PAGE: &str = r#"{"measures": ["sales.amount", "refunds.amount"],
    "dimensions": ["stores.store_name"], "segments": ["stores.open"],
    "filters": [{"member": "stores.store_name", "operator": "startsWith", "values": ["North"]},
                {"member": "refunds.amount", "operator": "gt", "values": ["0"]}],
    "order": [["refunds.amount", "desc"]], "limit": 5, "offset": 5}"#;
const REPEATED_SUM: &str = r#"{"measures": ["sales.amount", "stores.floor_area"],
    "dimensions": ["sales.channel_cd"], "order": [["sales.channel_cd", "asc"]]}"#;

// scenario: B1-MULTI-001
// scenario: B0-DET-001
#[test]
fn each_fact_is_aggregated_apart_and_combined_over_the_shared_dimension() {
    // With a dimension the facts meet on the distinct values either of them has;
    // without one each is a single row and the two are put side by side.
    insta::assert_snapshot!([BY_STORE, TOTALS]
        .map(|query| compiled(DialectId::Presto, query))
        .join("\n\n"));
}

// scenario: B1-MULTI-002
// scenario: B0-DET-001
#[test]
fn dimension_filters_reach_every_fact_and_measure_filters_the_combined_rows() {
    insta::assert_snapshot!(compiled(DialectId::Presto, FILTERED_PAGE));
    // The dimension filter is bound once in each fact, before the measure filter.
    assert_eq!(
        presto(FILTERED_PAGE).parameters,
        text(&["North", "North", "0"])
    );

    // A security filter is a dimension filter: no fact escapes it.
    let mut secured = context(DialectId::Presto);
    secured.security_filters = load_query(
        r#"{"filters": [{"member": "stores.store_name", "operator": "equals", "values": ["central"]}]}"#,
    )
    .unwrap()
    .filters;
    let compiled = compile_in(&secured, BY_STORE).expect("the query compiles");
    assert_eq!(
        compiled
            .sql
            .matches(r#"WHERE ("stores".store_name = ?)"#)
            .count(),
        2,
        "{}",
        compiled.sql
    );
    assert_eq!(compiled.parameters, text(&["central", "central"]));

    // A filter may name a measure the query does not select, of a third fact too.
    let hidden = presto(
        r#"{"measures": ["sales.amount"], "dimensions": ["stores.store_name"],
            "filters": [{"member": "refunds.amount", "operator": "gt", "values": ["0"]}]}"#,
    );
    let aliases: Vec<_> = hidden.columns.iter().map(|c| c.alias.as_str()).collect();
    assert_eq!(aliases, ["stores__store_name", "sales__amount"]);
    assert!(
        hidden.sql.contains(r#"WHERE ("q_1"."refunds__amount" > "#),
        "{}",
        hidden.sql
    );
}

// scenario: B1-MULTI-003
// scenario: B0-DET-001
#[test]
fn measure_over_repeated_rows_is_aggregated_over_distinct_keys() {
    // Alone, and beside a measure of the cube the join starts from.
    insta::assert_snapshot!([
        r#"{"measures": ["stores.floor_area"], "dimensions": ["sales.channel_cd"],
            "order": [["sales.channel_cd", "asc"]]}"#,
        REPEATED_SUM,
    ]
    .map(|query| compiled(DialectId::Presto, query))
    .join("\n\n"));
    // Every aggregate that repeated rows could change goes the same way; counts do not.
    for measure in ["avg_area", "min_area", "max_area"] {
        let sql = presto(&format!(
            r#"{{"measures": ["stores.{measure}"], "dimensions": ["sales.channel_cd"]}}"#
        ))
        .sql;
        assert!(
            sql.contains(r#"SELECT DISTINCT "stores_key_sales".channel_cd"#),
            "{sql}"
        );
    }
    let counts = presto(
        r#"{"measures": ["sales.amount", "stores.store_count"], "dimensions": ["sales.channel_cd"]}"#,
    );
    assert!(
        counts.sql.contains(r#"COUNT(DISTINCT "stores".store_cd)"#) && !counts.sql.contains("WITH"),
        "{}",
        counts.sql
    );
}

// scenario: B1-MULTI-004
#[test]
fn measures_that_one_select_can_hold_stay_in_one_select() {
    // A one_to_one join repeats no row, so both cubes' measures aggregate together.
    let flat = presto(
        r#"{"measures": ["sales.amount", "receipts.tax"], "dimensions": ["sales.channel_cd"]}"#,
    );
    assert!(
        flat.sql.contains(
            r#"sum("sales".amount) "sales__amount", sum("receipts".tax) "receipts__tax""#
        ) && !flat.sql.contains("WITH"),
        "{}",
        flat.sql
    );
}

// scenario: B1-MULTI-005
#[test]
fn fact_that_cannot_reach_what_the_query_reads_is_a_query_error() {
    let refused = |query: &str| match compile_in(&context(DialectId::Presto), query) {
        Err(CompileError::Query(message)) => message,
        other => panic!("expected a query failure, got {other:?}"),
    };
    // A dimension of one fact is not a dimension of the other.
    assert_eq!(
        refused(
            r#"{"measures": ["sales.amount", "refunds.amount"], "dimensions": ["sales.channel_cd"]}"#
        ),
        "No join path connects cubes `refunds`, `sales`: none of them reaches all the others"
    );
    // Neither is its time dimension, nor a filter on it.
    for rest in [
        r#""timeDimensions": [{"dimension": "sales.sold_at", "granularity": "day"}]"#,
        r#""filters": [{"member": "sales.channel_cd", "operator": "equals", "values": ["C1"]}]"#,
    ] {
        let query = format!(r#"{{"measures": ["sales.amount", "refunds.amount"], {rest}}}"#);
        assert!(refused(&query).contains("No join path connects"), "{rest}");
    }
    // Each fact is held to the rules of a query of its own: one path, one data source.
    assert!(refused(
        r#"{"measures": ["sales.amount", "refunds.amount"], "dimensions": ["regions.region_name"]}"#
    )
    .contains("more than one join path"));
    assert!(refused(
        r#"{"measures": ["sales.amount", "refunds.amount"], "dimensions": ["visits.source"]}"#
    )
    .contains("different data sources"));
}

// scenario: B1-MULTI-008
#[test]
fn one_tree_over_the_whole_query_serves_a_fact_that_reaches_nothing_alone() {
    // `receipts` declares no join: `sales` joins it one_to_one. Grouped by a store
    // dimension, which `sales` also reaches, the query is one tree from `sales`,
    // and Cube writes it as one SELECT.
    let by_store = presto(
        r#"{"measures": ["sales.amount", "receipts.tax"], "dimensions": ["stores.store_name"],
            "order": [["stores.store_name", "asc"]]}"#,
    );
    assert_eq!(
        by_store.sql,
        "SELECT \"stores\".store_name \"stores__store_name\", sum(\"sales\".amount) \"sales__amount\", \
         sum(\"receipts\".tax) \"receipts__tax\"  \nFROM  dw.sales_dtl  AS \"sales\"\n\
         LEFT JOIN  dw.dim_store  AS \"stores\" ON \"sales\".store_cd = \"stores\".store_cd\n\
         LEFT JOIN  dw.receipt  AS \"receipts\" ON \"sales\".receipt_no = \"receipts\".receipt_no \
         GROUP BY 1 ORDER BY  1  ASC"
    );
    let used: Vec<_> = by_store
        .used_cubes
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert_eq!(used, ["receipts", "sales", "stores"]);
    // Two joins away, and with a filter on the dimension.
    assert!(compile_in(
        &context(DialectId::Doris),
        r#"{"measures": ["sales.amount", "receipts.receipt_count"], "dimensions": ["regions.region_name"],
            "filters": [{"member": "stores.store_name", "operator": "set"}]}"#
    )
    .is_ok());
    // Without a cube of the query that joins it, the fact is still unreachable,
    // in Cube as here: no measure or dimension brings `sales` in.
    for measures in [r#""receipts.tax""#, r#""receipts.tax", "refunds.amount""#] {
        let query = format!(r#"{{"measures": [{measures}], "dimensions": ["stores.store_name"]}}"#);
        match compile_in(&context(DialectId::Presto), &query) {
            Err(CompileError::Query(message)) => assert_eq!(
                message,
                "No join path connects cubes `receipts`, `stores`: none of them reaches all the others"
            ),
            other => panic!("expected a query failure, got {other:?}"),
        }
    }
}

// scenario: B1-MULTI-006
#[test]
fn filter_on_a_count_over_repeated_rows_is_unsupported_by_name() {
    let unsupported = |query: &str| match compile_in(&context(DialectId::Presto), query) {
        Err(CompileError::Unsupported(message)) => message,
        other => panic!("expected Unsupported, got {other:?}"),
    };
    let filter =
        r#""filters": [{"member": "stores.store_count", "operator": "gt", "values": ["1"]}]"#;
    // Selected or not, grouped or not, beside another fact or alone.
    for rest in [
        r#""measures": ["sales.amount"]"#,
        r#""measures": ["sales.amount", "stores.store_count"], "dimensions": ["sales.channel_cd"]"#,
        r#""measures": ["stores.store_count"], "dimensions": ["sales.channel_cd"]"#,
        r#""measures": ["stores.floor_area"], "dimensions": ["sales.channel_cd"]"#,
    ] {
        assert_eq!(
            unsupported(&format!("{{{rest}, {filter}}}")),
            "The filter on `stores.store_count` compares the count `stores.store_count`, and \
             this query reaches `stores` through a many_to_one join; a filter on a count over \
             repeated rows is not supported",
            "{rest}"
        );
    }
    // A formula computed from the count compares the same repeated rows.
    let model = RETAIL.replace(
        "- { name: store_count, type: count }",
        "- { name: store_count, type: count }\n      \
         - { name: store_pairs, type: number, sql: \"{store_count} / 2\" }",
    );
    let query = load_query(
        r#"{"measures": ["sales.amount"],
            "filters": [{"member": "stores.store_pairs", "operator": "gt", "values": ["1"]}]}"#,
    )
    .unwrap();
    let model = Arc::new(load_yaml(&model).expect("the model loads"));
    match compile(&model, &query, &context(DialectId::Presto)) {
        Err(CompileError::Unsupported(message)) => assert!(
            message.contains("`stores.store_pairs`") && message.contains("`stores.store_count`"),
            "{message}"
        ),
        other => panic!("expected Unsupported, got {other:?}"),
    }
    // No join repeats the rows of the cube the query starts from, or of a one_to_one cube.
    for query in [
        r#"{"measures": ["stores.floor_area"], "dimensions": ["stores.store_name"],
            "filters": [{"member": "stores.store_count", "operator": "gt", "values": ["1"]}]}"#,
        r#"{"measures": ["sales.amount"], "dimensions": ["sales.channel_cd"],
            "filters": [{"member": "receipts.receipt_count", "operator": "gt", "values": ["1"]}]}"#,
        // A distinct count is the same over repeated rows.
        r#"{"measures": ["sales.amount"], "dimensions": ["sales.channel_cd"],
            "filters": [{"member": "stores.name_count", "operator": "gt", "values": ["1"]}]}"#,
    ] {
        assert!(
            compile_in(&context(DialectId::Presto), query).is_ok(),
            "{query}"
        );
    }
}

// scenario: B1-MULTI-007
#[test]
fn columns_and_lineage_cover_every_fact() {
    let combined = presto(BY_STORE);
    let columns: Vec<_> = combined
        .columns
        .iter()
        .map(|column| (column.alias.as_str(), column.member.as_str()))
        .collect();
    assert_eq!(
        columns,
        [
            ("stores__store_name", "stores.store_name"),
            ("sales__amount", "sales.amount"),
            ("refunds__amount", "refunds.amount"),
        ]
    );
    let lineage = |compiled: &Compiled| -> Vec<(String, Vec<String>)> {
        let cubes = compiled.used_cubes.iter();
        cubes
            .map(|cube| {
                let tables = cube.physical_tables.iter();
                (cube.name.clone(), tables.map(|t| t.join(".")).collect())
            })
            .collect()
    };
    let table = |cube: &str, table: &str| (cube.to_owned(), vec![table.to_owned()]);
    assert_eq!(
        lineage(&combined),
        [
            table("refunds", "dw.refund_dtl"),
            table("sales", "dw.sales_dtl"),
            table("stores", "dw.dim_store"),
        ]
    );
    // In the subquery of distinct keys the planner gives the cubes other aliases.
    let keyed =
        presto(r#"{"measures": ["stores.floor_area"], "dimensions": ["sales.channel_cd"]}"#);
    assert_eq!(
        lineage(&keyed),
        [
            table("sales", "dw.sales_dtl"),
            table("stores", "dw.dim_store")
        ]
    );
    // A fact only a filter names is read, and is not a column.
    let hidden = presto(
        r#"{"measures": ["sales.amount"],
            "filters": [{"member": "refunds.amount", "operator": "gt", "values": ["0"]}]}"#,
    );
    assert_eq!(
        lineage(&hidden),
        [
            table("refunds", "dw.refund_dtl"),
            table("sales", "dw.sales_dtl")
        ]
    );
    assert_eq!(hidden.columns.len(), 1);
}

// scenario: B1-DORIS-001
// scenario: B0-DET-001
#[test]
fn doris_combines_facts_with_its_own_quoting_and_paging() {
    insta::assert_snapshot!([BY_STORE, TOTALS, FILTERED_PAGE, REPEATED_SUM]
        .map(|query| compiled(DialectId::Doris, query))
        .join("\n\n"));
    // Same parameters, columns and lineage as Presto for the same query.
    for query in [BY_STORE, TOTALS, FILTERED_PAGE, REPEATED_SUM] {
        let doris = compile_in(&context(DialectId::Doris), query).expect("the query compiles");
        let presto = presto(query);
        assert_eq!(doris.parameters, presto.parameters, "{query}");
        assert_eq!(doris.columns, presto.columns, "{query}");
        assert_eq!(doris.used_cubes, presto.used_cubes, "{query}");
    }
}
