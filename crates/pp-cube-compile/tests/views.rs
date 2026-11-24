//! Views, compiled for Presto and Doris.
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

const MODEL: &str = include_str!("fixtures/views.yaml");

fn context(dialect: DialectId) -> CompileContext {
    CompileContext {
        request_id: String::new(),
        dialect,
        timezone: "UTC".to_owned(),
        security_filters: Vec::new(),
    }
}

fn compile_in(context: &CompileContext, query: &str) -> Result<Compiled, CompileError> {
    let model = Arc::new(load_yaml(MODEL).expect("the model loads"));
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

const BY_STORE: &str = r#"{"measures": ["sales_view.amount"],
    "dimensions": ["sales_view.channel_cd", "sales_view.stores_store_name"],
    "order": [["sales_view.channel_cd", "asc"]]}"#;
const BY_REGION_PAGE: &str = r#"{"measures": ["sales_view.amount", "sales_view.sale_count"],
    "dimensions": ["sales_view.region_name"],
    "timeDimensions": [{"dimension": "sales_view.sold_at", "granularity": "month", "dateRange": ["2026-01-01", "2026-12-31"]}],
    "filters": [{"member": "sales_view.stores_store_name", "operator": "equals", "values": ["central"]},
                {"member": "sales_view.amount", "operator": "gt", "values": ["10"]}],
    "order": [["sales_view.amount", "desc"]], "limit": 5, "offset": 5}"#;
const REPEATED_SUM: &str = r#"{"measures": ["sales_view.amount", "sales_view.stores_floor_area"],
    "dimensions": ["sales_view.channel_cd"], "order": [["sales_view.channel_cd", "asc"]]}"#;

/// The same query written against the cubes the view takes its members from.
fn on_cubes(query: &str) -> String {
    query
        .replace("sales_view.stores_store_name", "stores.store_name")
        .replace("sales_view.stores_floor_area", "stores.floor_area")
        .replace("sales_view.region_name", "regions.region_name")
        .replace("sales_view.tax", "receipts.tax")
        .replace("sales_view.", "sales.")
}

// scenario: B1-VIEW-001
// scenario: B0-DET-001
#[test]
fn view_members_compile_as_the_members_they_take_under_the_views_name() {
    insta::assert_snapshot!([BY_STORE, BY_REGION_PAGE]
        .map(|query| compiled(DialectId::Presto, query))
        .join("\n\n"));
    // The columns are the view's; the cubes read are the ones behind it.
    let by_store = presto(BY_STORE);
    let columns: Vec<_> = by_store
        .columns
        .iter()
        .map(|column| (column.alias.as_str(), column.member.as_str()))
        .collect();
    assert_eq!(
        columns,
        [
            ("sales_view__channel_cd", "sales_view.channel_cd"),
            (
                "sales_view__stores_store_name",
                "sales_view.stores_store_name"
            ),
            ("sales_view__amount", "sales_view.amount"),
        ]
    );
    let used: Vec<_> = by_store
        .used_cubes
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert_eq!(used, ["sales", "stores"]);
    // But for the names of its columns, it is the query on the cubes.
    for query in [BY_STORE, BY_REGION_PAGE] {
        let view = presto(query);
        let cubes = presto(&on_cubes(query));
        let renamed = view
            .sql
            .replace("sales_view__stores_store_name", "stores__store_name")
            .replace("sales_view__region_name", "regions__region_name")
            .replace("sales_view__", "sales__");
        assert_eq!(renamed, cubes.sql, "{query}");
        assert_eq!(view.parameters, cubes.parameters, "{query}");
        assert_eq!(view.used_cubes, cubes.used_cubes, "{query}");
    }
    // A view member and a cube member may stand in one query.
    assert!(compile_in(
        &context(DialectId::Presto),
        r#"{"measures": ["sales.amount"], "dimensions": ["sales_view.stores_store_name"]}"#
    )
    .is_ok());
}

// scenario: B1-VIEW-002
// scenario: B0-DET-001
#[test]
fn everything_a_member_can_do_it_does_through_a_view() {
    // A sum over rows the view's own join repeats.
    insta::assert_snapshot!(compiled(DialectId::Presto, REPEATED_SUM));
    let over = |measures: &str, time: &str| {
        format!(
            r#"{{"measures": [{measures}], "timeDimensions": [{{"dimension": "sales_view.sold_at", {time}}}]}}"#
        )
    };
    let year = r#""granularity": "month", "dateRange": ["2026-01-01", "2026-12-31"]"#;
    for (query, fragment) in [
        // A rolling window, a time shift along the view's time dimension, a
        // declared granularity, a formula and a filtered sum beside a measure of
        // another cube, and rows.
        (
            over(r#""sales_view.amount_7d""#, r#""granularity": "day", "dateRange": ["2026-10-01", "2026-10-03"]"#),
            r#"LEFT JOIN  cte_0  AS "rolling_source" ON "#,
        ),
        (
            over(r#""sales_view.amount", "sales_view.amount_prior_year""#, year),
            r#"("sales".sold_at + interval '1' year)"#,
        ),
        (
            over(r#""sales_view.amount""#, r#""granularity": "fiscal_year""#),
            r#"date_trunc('year', "#,
        ),
        (
            r#"{"measures": ["sales_view.per_sale", "sales_view.toy_amount", "sales_view.tax"]}"#.to_owned(),
            r#"sum("sales".amount) / NULLIF(count("sales".sale_id), 0) "sales__per_sale""#,
        ),
        (
            r#"{"ungrouped": true, "dimensions": ["sales_view.sale_id"], "measures": ["sales_view.amount"]}"#.to_owned(),
            r#""sales".amount "sales_view__amount""#,
        ),
    ] {
        let sql = presto(&query).sql;
        assert!(sql.contains(fragment), "{query}\n{sql}");
    }
}

// scenario: B1-VIEW-003
#[test]
fn view_keeps_the_rules_of_the_members_behind_it() {
    let failure = |context: &CompileContext, query: &str| compile_in(context, query).unwrap_err();
    let presto_context = context(DialectId::Presto);
    // Only what the view takes is in it.
    for member in [
        "sales_view.missing",
        "sales_view.receipt_no",
        "sales_view.store_name",
    ] {
        let query = format!(r#"{{"dimensions": ["{member}"], "measures": ["sales_view.amount"]}}"#);
        assert!(
            matches!(failure(&presto_context, &query), CompileError::Query(message) if message.contains(member)),
            "{member}"
        );
    }
    // The refusals of the members behind it hold.
    let count_filter = r#"{"measures": ["sales_view.amount"],
        "filters": [{"member": "sales_view.stores_store_count", "operator": "gt", "values": ["1"]}]}"#;
    assert!(matches!(
        failure(&presto_context, count_filter),
        CompileError::Unsupported(message) if message.contains("stores.store_count")
    ));
    let no_time = r#"{"measures": ["sales_view.amount_7d"]}"#;
    assert!(matches!(
        failure(&presto_context, no_time),
        CompileError::Unsupported(message) if message.contains("sales.amount_7d")
    ));
    // A security filter on a view member restricts the cube behind it.
    let mut secured = context(DialectId::Presto);
    secured.security_filters = load_query(
        r#"{"filters": [{"member": "sales_view.region_name", "operator": "equals", "values": ["north"]}]}"#,
    )
    .unwrap()
    .filters;
    let compiled = compile_in(&secured, r#"{"measures": ["sales_view.amount"]}"#).unwrap();
    assert!(
        compiled
            .sql
            .contains(r#"WHERE ("regions".region_name = ?)"#),
        "{}",
        compiled.sql
    );
    assert_eq!(compiled.parameters, [Parameter::Text("north".into())]);
}

// scenario: B1-DORIS-004
// scenario: B0-DET-001
#[test]
fn doris_compiles_views_with_its_own_quoting() {
    insta::assert_snapshot!([BY_STORE, BY_REGION_PAGE, REPEATED_SUM]
        .map(|query| compiled(DialectId::Doris, query))
        .join("\n\n"));
    for query in [BY_STORE, BY_REGION_PAGE, REPEATED_SUM] {
        let doris = compile_in(&context(DialectId::Doris), query).expect("the query compiles");
        let presto = presto(query);
        assert_eq!(doris.parameters, presto.parameters, "{query}");
        assert_eq!(doris.columns, presto.columns, "{query}");
        assert_eq!(doris.used_cubes, presto.used_cubes, "{query}");
    }
}
