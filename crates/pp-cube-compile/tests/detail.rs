//! Ungrouped rows, approximate distinct counts and granularities a time
//! dimension declares, compiled for Presto and Doris.
//!
//! The snapshots are the SQL the pinned planner emits with each dialect's
//! templates, followed by the parameters in placeholder order. The Presto text
//! of every query here was compared with Cube Core at the release of the pin;
//! execution belongs to the engine tiers.

mod support;
use support::compile;

use std::sync::Arc;

use pp_cube_compile::{CompileContext, CompileError, Compiled, DialectId};
use pp_cube_model::{load_query, load_yaml};

const MODEL: &str = include_str!("fixtures/detail.yaml");

fn compile_in(dialect: DialectId, query: &str) -> Result<Compiled, CompileError> {
    let model = Arc::new(load_yaml(MODEL).expect("the model loads"));
    let query = load_query(query).expect("the query loads");
    let context = CompileContext {
        request_id: String::new(),
        dialect,
        timezone: "UTC".to_owned(),
        security_filters: Vec::new(),
    };
    compile(&model, &query, &context)
}

fn presto(query: &str) -> Compiled {
    compile_in(DialectId::Presto, query).expect("the query compiles")
}

/// SQL and parameters of `query` in `dialect`, as one text to snapshot.
fn compiled(dialect: DialectId, query: &str) -> String {
    let compiled = compile_in(dialect, query).expect("the query compiles");
    format!("{}\n-- parameters: {:?}", compiled.sql, compiled.parameters)
}

fn unsupported(query: &str) -> String {
    match compile_in(DialectId::Presto, query) {
        Err(CompileError::Unsupported(message)) => message,
        other => panic!("expected Unsupported, got {other:?}"),
    }
}

fn query_failure(query: &str) -> String {
    match compile_in(DialectId::Presto, query) {
        Err(CompileError::Query(message)) => message,
        other => panic!("expected a query failure, got {other:?}"),
    }
}

const ROWS: &str = r#"{"ungrouped": true, "dimensions": ["sales.sale_id"],
    "measures": ["sales.amount", "sales.sale_count", "sales.buyers", "sales.toy_amount"]}"#;
const JOINED_ROWS: &str = r#"{"ungrouped": true, "dimensions": ["sales.sale_id", "stores.store_name"],
    "measures": ["sales.amount"],
    "timeDimensions": [{"dimension": "sales.sold_at", "granularity": "day", "dateRange": ["2026-10-01", "2026-10-31"]}],
    "filters": [{"member": "sales.channel_cd", "operator": "equals", "values": ["C1"]}],
    "order": [["sales.sale_id", "asc"]], "limit": 5, "offset": 5}"#;
const APPROXIMATE: &str = r#"{"measures": ["sales.buyers_approx", "sales.per_buyer"],
    "dimensions": ["sales.channel_cd"],
    "filters": [{"member": "sales.buyers_approx", "operator": "gt", "values": ["1"]}],
    "order": [["sales.channel_cd", "asc"]]}"#;

fn by(granularity: &str) -> String {
    format!(
        r#"{{"measures": ["sales.amount"], "timeDimensions": [{{"dimension": "sales.sold_at",
            "granularity": "{granularity}", "dateRange": ["2026-01-01", "2026-12-31"]}}]}}"#
    )
}

// scenario: B1-DETAIL-001
// scenario: B0-DET-001
#[test]
fn ungrouped_query_returns_rows_with_measures_unaggregated() {
    insta::assert_snapshot!([ROWS, JOINED_ROWS]
        .map(|query| compiled(DialectId::Presto, query))
        .join("\n\n"));
    let rows = presto(JOINED_ROWS);
    let columns: Vec<_> = rows
        .columns
        .iter()
        .map(|column| (column.alias.as_str(), column.member.as_str()))
        .collect();
    assert_eq!(
        columns,
        [
            ("sales__sale_id", "sales.sale_id"),
            ("stores__store_name", "stores.store_name"),
            ("sales__sold_at_day", "sales.sold_at"),
            ("sales__amount", "sales.amount"),
        ]
    );
    assert!(!rows.sql.contains("GROUP BY"), "{}", rows.sql);
    // `false` is the grouped query it always was.
    let grouped = presto(r#"{"ungrouped": false, "measures": ["sales.amount"]}"#);
    assert_eq!(grouped.sql, presto(r#"{"measures": ["sales.amount"]}"#).sql);
}

// scenario: B1-DETAIL-002
#[test]
fn ungrouped_query_refuses_what_rows_cannot_carry() {
    // No aggregate to compare.
    assert_eq!(
        query_failure(
            r#"{"ungrouped": true, "dimensions": ["sales.sale_id"],
                "filters": [{"member": "sales.amount", "operator": "gt", "values": ["10"]}]}"#
        ),
        "Filter member `sales.amount` is a measure; an ungrouped query cannot be filtered on a measure"
    );
    // The planner aggregates these apart, so part of the result would be grouped.
    let repeated = r#"{"ungrouped": true, "dimensions": ["sales.sale_id"], "measures": ["stores.floor_area"]}"#;
    assert_eq!(
        unsupported(repeated),
        "An ungrouped query cannot take measures that need an aggregation of their own: \
         measures of several cubes, over rows a join repeats, rolling or shifted"
    );
    let rolling = r#"{"ungrouped": true, "dimensions": ["sales.sale_id"], "measures": ["sales.amount_7d"],
        "timeDimensions": [{"dimension": "sales.sold_at", "granularity": "day", "dateRange": ["2026-10-01", "2026-10-03"]}]}"#;
    assert!(unsupported(rolling).starts_with("An ungrouped query cannot take measures"));
}

// scenario: B1-APPROX-001
// scenario: B0-DET-001
#[test]
fn approximate_distinct_count_uses_the_engines_estimate() {
    // Selected, in a formula and in a filter; then beside a sum over repeated rows.
    insta::assert_snapshot!([
        APPROXIMATE,
        r#"{"measures": ["sales.buyers_approx", "stores.floor_area"], "dimensions": ["stores.store_name"]}"#,
    ]
    .map(|query| compiled(DialectId::Presto, query))
    .join("\n\n"));
    // The exact count is still exact.
    let exact = presto(r#"{"measures": ["sales.buyers"]}"#).sql;
    assert!(
        exact.contains(r#"COUNT(DISTINCT "sales".buyer_id)"#),
        "{exact}"
    );
}

// scenario: B1-GRAN-001
// scenario: B0-DET-001
#[test]
fn declared_granularity_counts_periods_from_its_origin_or_moves_natural_ones() {
    insta::assert_snapshot!(["half_year", "fortnight", "fiscal_year", "month_from_15"]
        .map(|granularity| compiled(DialectId::Presto, &by(granularity)))
        .join("\n\n"));
    // The column names the granularity it was grouped by.
    let column = &presto(&by("half_year")).columns[0];
    assert_eq!(
        (
            column.alias.as_str(),
            column.member.as_str(),
            column.granularity.as_deref()
        ),
        (
            "sales__sold_at_half_year",
            "sales.sold_at",
            Some("half_year")
        )
    );
    // The origin is a local time, as the dimension is in the query's timezone.
    let tokyo =
        by("half_year").replace(r#"{"measures""#, r#"{"timezone": "Asia/Tokyo", "measures""#);
    let sql = presto(&tokyo).sql;
    assert!(
        sql.contains("AT TIME ZONE 'Asia/Tokyo'")
            && sql.contains("from_iso8601_timestamp('2026-01-01T00:00:00.000')"),
        "{sql}"
    );
}

// scenario: B1-GRAN-002
#[test]
fn granularity_a_dimension_does_not_declare_or_cannot_roll_by_is_refused() {
    assert_eq!(
        query_failure(&by("decade")),
        "Granularity `decade` is not one `sales.sold_at` declares"
    );
    // Declared by one dimension is not declared by another.
    assert!(query_failure(
        r#"{"measures": ["sales.amount"], "dimensions": ["stores.store_name"],
            "timeDimensions": [{"dimension": "sales.sale_id", "granularity": "half_year"}]}"#
    )
    .contains("`timeDimensions` takes time dimensions"));
    assert_eq!(
        unsupported(&by("half_year").replace("sales.amount", "sales.amount_7d")),
        "Measure `sales.amount_7d` has a rolling window, and it cannot roll by `half_year`, a \
         granularity `sales.sold_at` declares"
    );
    // Periods counted from an origin are wrong for rows before it, so the range
    // has to start there or later; natural periods that are only moved have no
    // such limit.
    let from = |granularity: &str, time: &str| {
        format!(
            r#"{{"measures": ["sales.amount"], "timeDimensions": [{{"dimension": "sales.sold_at",
                "granularity": "{granularity}"{time}}}]}}"#
        )
    };
    for time in ["", r#", "dateRange": ["2025-12-31", "2026-12-31"]"#] {
        assert_eq!(
            unsupported(&from("half_year", time)),
            "Granularity `half_year` of `sales.sold_at` counts its periods from `2026-01-01`, and \
             rows before that would be put in the wrong period; give `sales.sold_at` a date range \
             that starts there or later",
            "{time}"
        );
    }
    for granularity in ["fiscal_year", "month_from_15"] {
        assert!(
            compile_in(DialectId::Presto, &from(granularity, "")).is_ok(),
            "{granularity}"
        );
    }
    // Ordered by, and paged, as any time dimension.
    let ordered = by("fortnight").replace(
        r#"{"measures""#,
        r#"{"order": [["sales.sold_at", "desc"]], "limit": 3, "measures""#,
    );
    assert!(presto(&ordered).sql.ends_with("ORDER BY  1  DESC\nLIMIT 3"));
}

// scenario: B1-DORIS-003
// scenario: B0-DET-001
#[test]
fn doris_writes_rows_estimates_and_declared_periods_with_its_own_functions() {
    let queries = [
        ROWS.to_owned(),
        JOINED_ROWS.to_owned(),
        APPROXIMATE.to_owned(),
        by("half_year"),
        by("fortnight"),
        by("fiscal_year"),
    ];
    insta::assert_snapshot!(queries
        .each_ref()
        .map(|query| compiled(DialectId::Doris, query))
        .join("\n\n"));
    for query in &queries {
        let doris = compile_in(DialectId::Doris, query).expect("the query compiles");
        let presto = presto(query);
        assert_eq!(doris.parameters, presto.parameters, "{query}");
        assert_eq!(doris.columns, presto.columns, "{query}");
        assert_eq!(doris.used_cubes, presto.used_cubes, "{query}");
    }
}
