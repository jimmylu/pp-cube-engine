//! The Druid dialect: what it compiles, what it declares, what it refuses.
//!
//! The SQL of every Druid execution case is a golden of `pp-cube-conformance`,
//! compared with Cube Core's Druid dialect and executed on Druid 26.

mod support;
use support::compile;

use std::sync::Arc;

use pp_cube_compile::{CompileContext, CompileError, Compiled, DialectId, SessionProperty};
use pp_cube_model::{load_query, load_yaml};

const RETAIL: &str = include_str!("fixtures/retail.yaml");
const WINDOWS: &str = include_str!("fixtures/windows.yaml");
const DETAIL: &str = include_str!("fixtures/detail.yaml");

fn compile_in(model: &str, dialect: DialectId, query: &str) -> Result<Compiled, CompileError> {
    let model = Arc::new(load_yaml(model).expect("the model loads"));
    let query = load_query(query).expect("the query loads");
    let context = CompileContext {
        request_id: String::new(),
        dialect,
        timezone: "Asia/Tokyo".to_owned(),
        security_filters: Vec::new(),
    };
    compile(&model, &query, &context)
}

fn druid(model: &str, query: &str) -> Result<Compiled, CompileError> {
    compile_in(model, DialectId::Druid, query)
}

// scenario: B0-DRUID-001
// scenario: B0-DET-001
#[test]
fn druid_writes_cubes_druid_dialect() {
    // Joined dimensions, a segment, filters on a dimension and a measure, time
    // in a zone, order and a page: one statement in Druid's functions.
    let compiled = druid(
        RETAIL,
        r#"{"measures": ["sales.amount"], "dimensions": ["stores.store_name"], "segments": ["stores.open"],
            "timeDimensions": [{"dimension": "sales.sold_at", "granularity": "month", "dateRange": ["2026-10-01", "2026-10-31"]}],
            "filters": [{"member": "stores.store_name", "operator": "contains", "values": ["North"]},
                        {"member": "sales.amount", "operator": "gt", "values": ["10"]}],
            "order": [["sales.amount", "desc"]], "limit": 5, "offset": 5}"#,
    )
    .expect("the query compiles");
    insta::assert_snapshot!(format!(
        "{}\n-- parameters: {:?}",
        compiled.sql, compiled.parameters
    ));
    // The same query has the same parameters, columns and lineage on Presto.
    let query = r#"{"measures": ["sales.amount"], "dimensions": ["stores.store_name"],
        "timeDimensions": [{"dimension": "sales.sold_at", "granularity": "day", "dateRange": ["2026-10-01", "2026-10-31"]}]}"#;
    let (on_druid, on_presto) = (
        druid(RETAIL, query).unwrap(),
        compile_in(RETAIL, DialectId::Presto, query).unwrap(),
    );
    assert_eq!(on_druid.parameters, on_presto.parameters);
    assert_eq!(on_druid.columns, on_presto.columns);
    assert_eq!(on_druid.used_cubes, on_presto.used_cubes);
    assert_ne!(on_druid.sql, on_presto.sql);
    assert_ne!(on_druid.fingerprint, on_presto.fingerprint);
}

// scenario: B0-DRUID-005
#[test]
fn druid_declares_exact_distinct_counts_for_every_statement() {
    // Druid estimates COUNT(DISTINCT) unless the query context says otherwise.
    let exact = [SessionProperty {
        name: "useApproximateCountDistinct".into(),
        value: "false".into(),
    }];
    for query in [
        r#"{"measures": ["stores.name_count"]}"#,
        r#"{"measures": ["sales.amount"], "limit": 2, "offset": 2}"#,
    ] {
        assert_eq!(
            druid(RETAIL, query).unwrap().session_properties,
            exact,
            "{query}"
        );
    }
    // Presto's own property is still only for a page.
    let presto = compile_in(
        RETAIL,
        DialectId::Presto,
        r#"{"measures": ["sales.amount"]}"#,
    );
    assert!(presto.unwrap().session_properties.is_empty());
}

// scenario: B0-DRUID-002
#[test]
fn druid_refuses_by_name_what_it_cannot_answer() {
    let ungrouped = r#"{"ungrouped": true, "dimensions": ["sales.sale_id"]}"#;
    match druid(DETAIL, ungrouped) {
        Err(CompileError::Unsupported(message)) => assert_eq!(
            message,
            "The query is ungrouped, which druid does not support"
        ),
        other => panic!("expected Unsupported, got {other:?}"),
    }
    // The other dialects compile it.
    for dialect in [DialectId::Presto, DialectId::Doris] {
        assert!(
            compile_in(DETAIL, dialect, ungrouped).is_ok(),
            "{dialect:?}"
        );
    }
    // What a model or a query gets wrong is still a query error on Druid.
    assert!(matches!(
        druid(RETAIL, r#"{"measures": ["sales.missing"]}"#),
        Err(CompileError::Query(_))
    ));
    assert!(matches!(
        druid(RETAIL, r#"{"measures": ["refunds.amount"], "dimensions": ["regions.region_name"]}"#),
        Err(CompileError::Query(message)) if message.contains("more than one join path")
    ));
}

// scenario: B1-DRUID-001
#[test]
fn druid_estimates_distinct_counts_and_groups_by_declared_granularities() {
    // Cube's Druid dialect has no SQL for the first four and SQL Druid rejects
    // for the fifth; these are the kernel's own, each one SELECT.
    let by = |granularity: &str| {
        format!(
            r#"{{"measures": ["sales.amount"], "timeDimensions": [{{"dimension": "sales.sold_at",
                "granularity": "{granularity}", "dateRange": ["2026-01-15", "2026-12-31"]}}]}}"#
        )
    };
    let queries = [
        r#"{"measures": ["sales.per_buyer"], "dimensions": ["sales.channel_cd"]}"#.to_owned(),
        by("half_year"),
        by("fortnight"),
        by("month_from_15"),
        by("fiscal_year"),
    ];
    let mut golden = String::new();
    for query in &queries {
        let on_druid = druid(DETAIL, query).expect("the query compiles");
        assert!(on_druid.sql.starts_with("SELECT"), "{query}");
        // The same question as on Presto: parameters, columns and lineage.
        let on_presto = compile_in(DETAIL, DialectId::Presto, query).unwrap();
        assert_eq!(on_druid.parameters, on_presto.parameters, "{query}");
        assert_eq!(on_druid.columns, on_presto.columns, "{query}");
        assert_eq!(on_druid.used_cubes, on_presto.used_cubes, "{query}");
        golden.push_str(&format!(
            "{}\n-- parameters: {:?}\n\n",
            on_druid.sql, on_druid.parameters
        ));
    }
    insta::assert_snapshot!(golden);
    // Druid's interval literal has no week: an offset in weeks is refused.
    let weekly = DETAIL.replace(
        r#"{ name: fiscal_year, interval: 1 year, offset: 3 month }"#,
        r#"{ name: fiscal_year, interval: 1 year, offset: 1 week }"#,
    );
    assert_ne!(weekly, DETAIL);
    match druid(&weekly, &by("fiscal_year")) {
        Err(CompileError::Unsupported(message)) => assert_eq!(
            message,
            "Granularity `fiscal_year` of `sales.sold_at` is offset by `1 week`, and druid has no interval of that unit"
        ),
        other => panic!("expected Unsupported, got {other:?}"),
    }
    // Presto's interval literal has no week either; Doris has one.
    assert!(matches!(
        compile_in(&weekly, DialectId::Presto, &by("fiscal_year")),
        Err(CompileError::Unsupported(_))
    ));
    assert!(compile_in(&weekly, DialectId::Doris, &by("fiscal_year")).is_ok());
}

// scenario: B1-DRUID-002
#[test]
fn druid_merges_several_selects_by_a_full_join_and_a_group() {
    // Cube's Druid dialect merges aggregates through a null-safe join over a
    // union of keys, which Druid rejects. Here they meet in a FULL JOIN on
    // plain equality and a GROUP BY of the merged keys.
    let days = r#""granularity": "day", "dateRange": ["2026-10-01", "2026-10-03"]"#;
    let shifted = format!(
        r#"{{"measures": ["sales.growth"], "timeDimensions": [{{"dimension": "sales.sold_at", {days}}}]}}"#
    );
    let queries = [
        // Two facts over a shared dimension.
        (
            RETAIL,
            r#"{"measures": ["sales.amount", "refunds.amount"], "dimensions": ["stores.store_name"]}"#,
        ),
        // A filter on a measure, an order and a page over the merged rows.
        // Three facts in a chain are goldens of `pp-cube-conformance`.
        (
            RETAIL,
            r#"{"measures": ["sales.amount", "refunds.amount"], "dimensions": ["stores.store_name"],
            "filters": [{"member": "refunds.amount", "operator": "gt", "values": ["0"]}],
            "order": [["sales.amount", "desc"]], "limit": 5, "offset": 5}"#,
        ),
        // A sum over rows a join repeats.
        (
            RETAIL,
            r#"{"measures": ["stores.floor_area"], "dimensions": ["sales.channel_cd"]}"#,
        ),
        // A measure beside itself a year earlier.
        (WINDOWS, shifted.as_str()),
        // Totals: no key to merge on, and no order to group for.
        (
            RETAIL,
            r#"{"measures": ["sales.amount", "refunds.amount"]}"#,
        ),
    ];
    let mut golden = String::new();
    for (model, query) in queries {
        let on_druid = druid(model, query).expect("the query compiles");
        assert!(on_druid.sql.starts_with(" WITH"), "{query}");
        // No condition Druid cannot join on, and no union of keys.
        assert!(!on_druid.sql.contains("IS NULL) AND"), "{query}");
        assert!(!on_druid.sql.contains("UNION"), "{query}");
        // The same question as on Presto: parameters, columns and lineage.
        let on_presto = compile_in(model, DialectId::Presto, query).unwrap();
        assert_eq!(on_druid.parameters, on_presto.parameters, "{query}");
        assert_eq!(on_druid.columns, on_presto.columns, "{query}");
        assert_eq!(on_druid.used_cubes, on_presto.used_cubes, "{query}");
        golden.push_str(&format!(
            "{}\n-- parameters: {:?}\n\n",
            on_druid.sql, on_druid.parameters
        ));
    }
    insta::assert_snapshot!(golden);
    // A shift by a unit Druid's interval literal lacks is refused, naming it.
    let weekly = WINDOWS.replace("interval: 1 year", "interval: 1 week");
    assert_ne!(weekly, WINDOWS);
    match druid(&weekly, &shifted) {
        Err(CompileError::Unsupported(message)) => {
            assert!(message.contains("`1 week`"), "{message}")
        }
        other => panic!("expected Unsupported, got {other:?}"),
    }
}

// scenario: B1-DRUID-003
#[test]
fn druid_rolls_a_window_by_a_cross_join_and_an_equality_join() {
    // The planner joins the time series to the aggregate on a range of time.
    // Druid joins on equality only: the series is crossed with the aggregate,
    // filtered and grouped, then joined back on the period.
    let over = |measure: &str, rest: &str| {
        format!(
            r#"{{"measures": ["{measure}"], {rest} "timeDimensions": [{{"dimension": "sales.sold_at",
                "granularity": "day", "dateRange": ["2026-10-01", "2026-10-03"]}}]}}"#
        )
    };
    let queries = [
        over("sales.amount_7d", ""),
        over("sales.amount_mtd", ""),
        // By a dimension: the aggregate's own column, beside the period.
        over("sales.amount_7d", r#""dimensions": ["sales.channel_cd"],"#),
    ];
    let mut golden = String::new();
    for query in &queries {
        let on_druid = druid(WINDOWS, query).expect("the query compiles");
        // No join on a range: the range is a filter over the crossed rows.
        assert!(
            on_druid.sql.contains("\"rolling_source\" ON 1 = 1"),
            "{query}"
        );
        assert!(
            on_druid
                .sql
                .contains("AS \"rolling\" ON \"time_series\".\"date_from\" = "),
            "{query}"
        );
        // The same question as on Doris, which also writes its periods out.
        let on_doris = compile_in(WINDOWS, DialectId::Doris, query).unwrap();
        assert_eq!(on_druid.parameters, on_doris.parameters, "{query}");
        assert_eq!(on_druid.columns, on_doris.columns, "{query}");
        assert_eq!(on_druid.used_cubes, on_doris.used_cubes, "{query}");
        golden.push_str(&format!(
            "{}\n-- parameters: {:?}\n\n",
            on_druid.sql, on_druid.parameters
        ));
    }
    insta::assert_snapshot!(golden);
    // The periods are written out, so rolling by a granularity needs a range.
    let unbounded = r#"{"measures": ["sales.amount_7d"],
        "timeDimensions": [{"dimension": "sales.sold_at", "granularity": "day"}]}"#;
    match druid(WINDOWS, unbounded) {
        Err(CompileError::Unsupported(message)) => assert_eq!(
            message,
            "Measure `sales.amount_7d` has a rolling window, and on druid `sales.sold_at` needs a date range to roll by its granularity"
        ),
        other => panic!("expected Unsupported, got {other:?}"),
    }
}
