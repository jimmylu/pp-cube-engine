//! Rolling windows, period to date and time shifts, compiled for Presto and Doris.
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

const MODEL: &str = include_str!("fixtures/windows.yaml");

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

fn unsupported(dialect: DialectId, query: &str) -> String {
    match compile_in(&context(dialect), query) {
        Err(CompileError::Unsupported(message)) => message,
        other => panic!("expected Unsupported, got {other:?}"),
    }
}

fn text(values: &[&str]) -> Vec<Parameter> {
    let values = values.iter();
    values
        .map(|value| Parameter::Text((*value).into()))
        .collect()
}

/// A query for `measures` over `sales.sold_at` with the rest of its time dimension.
fn over(measures: &str, time: &str) -> String {
    format!(
        r#"{{"measures": [{measures}], "timeDimensions": [{{"dimension": "sales.sold_at", {time}}}]}}"#
    )
}

const THREE_DAYS: &str = r#""granularity": "day", "dateRange": ["2026-10-01", "2026-10-03"]"#;
const OCTOBER: &str = r#""dateRange": ["2026-10-01", "2026-10-31"]"#;
const YEAR_BY_MONTH: &str = r#""granularity": "month", "dateRange": ["2026-01-01", "2026-12-31"]"#;

// scenario: B1-ROLL-001
// scenario: B0-DET-001
#[test]
fn window_is_summed_over_a_time_series_of_the_range() {
    // Trailing, beside the plain measure and a dimension; then a window around
    // the start of each month, and everything up to each month.
    insta::assert_snapshot!([
        format!(
            r#"{{"measures": ["sales.amount", "sales.amount_7d"], "dimensions": ["sales.channel_cd"],
                "timeDimensions": [{{"dimension": "sales.sold_at", {THREE_DAYS}}}]}}"#
        ),
        over(r#""sales.amount_around", "sales.amount_so_far""#, YEAR_BY_MONTH),
    ]
    .map(|query| compiled(DialectId::Presto, &query))
    .join("\n\n"));

    // The rows a window needs lie before the range; the source reaches back for them.
    let trailing = presto(&over(r#""sales.amount_7d""#, THREE_DAYS));
    assert_eq!(
        trailing.parameters[0],
        text(&["2026-09-24T00:00:00.000"])[0]
    );
    let column = &trailing.columns[0];
    assert_eq!(
        (
            column.alias.as_str(),
            column.member.as_str(),
            column.granularity.as_deref()
        ),
        ("sales__sold_at_day", "sales.sold_at", Some("day"))
    );
    assert_eq!(trailing.used_cubes.len(), 1);
}

// scenario: B1-ROLL-002
// scenario: B0-DET-001
#[test]
fn period_to_date_starts_at_the_period_of_each_row() {
    insta::assert_snapshot!([
        over(r#""sales.amount_mtd""#, THREE_DAYS),
        over(r#""sales.amount_ytd", "sales.amount""#, YEAR_BY_MONTH),
    ]
    .map(|query| compiled(DialectId::Presto, &query))
    .join("\n\n"));
}

// scenario: B1-ROLL-003
#[test]
fn without_a_granularity_the_window_ends_at_the_end_of_the_range() {
    let single = presto(&over(r#""sales.amount_7d""#, OCTOBER));
    assert!(
        single.sql.contains(
            r#"WHERE ("sales".sold_at > from_iso8601_timestamp(?) AND "sales".sold_at <= from_iso8601_timestamp(?))"#
        ) && !single.sql.contains("time_series"),
        "{}",
        single.sql
    );
    assert_eq!(
        single.parameters,
        text(&["2026-10-24T23:59:59.999Z", "2026-10-31T23:59:59.999Z"])
    );
    // Month to date at the end of the range is the month it ends in.
    let to_date = presto(&over(r#""sales.amount_mtd""#, OCTOBER));
    assert_eq!(
        to_date.parameters,
        text(&["2026-10-01T00:00:00.000Z", "2026-10-31T23:59:59.999Z"])
    );
}

// scenario: B1-ROLL-004
#[test]
fn averages_and_distinct_counts_roll_over_rows_not_over_daily_results() {
    let sql = presto(&over(r#""sales.avg_7d", "sales.buyers_7d""#, THREE_DAYS)).sql;
    // The source keeps one row per sale; the window aggregates them.
    for (source, rolled) in [
        (
            r#""sales".amount "sales__avg_7d""#,
            r#"avg("rolling_source"."sales__avg_7d")"#,
        ),
        (
            r#""sales".buyer_id "sales__buyers_7d""#,
            r#"COUNT(DISTINCT "rolling_source"."sales__buyers_7d")"#,
        ),
    ] {
        assert!(
            sql.contains(source) && sql.contains(rolled),
            "{source}\n{sql}"
        );
    }
}

// scenario: B1-SHIFT-001
// scenario: B0-DET-001
#[test]
fn shifted_measure_is_the_measure_with_time_moved_and_meets_the_current_one() {
    insta::assert_snapshot!([
        over(
            r#""sales.amount", "sales.amount_prior_year""#,
            YEAR_BY_MONTH
        ),
        over(r#""sales.amount_next_month""#, YEAR_BY_MONTH),
    ]
    .map(|query| compiled(DialectId::Presto, &query))
    .join("\n\n"));
    // The range is bound once for the shifted aggregate and once for the current one.
    let range = ["2026-01-01T00:00:00.000Z", "2026-12-31T23:59:59.999Z"];
    assert_eq!(
        presto(&over(
            r#""sales.amount", "sales.amount_prior_year""#,
            YEAR_BY_MONTH
        ))
        .parameters,
        text(&[range[0], range[1], range[0], range[1]])
    );
}

// scenario: B1-SHIFT-002
#[test]
fn formulas_and_filters_see_the_shifted_measure() {
    let growth = presto(&over(r#""sales.growth""#, YEAR_BY_MONTH));
    assert!(
        growth.sql.contains(
            r#"("q_0"."sales__amount" - "q_1"."sales__amount_prior_year") / NULLIF("q_1"."sales__amount_prior_year", 0) "sales__growth""#
        ),
        "{}",
        growth.sql
    );
    // Dimension filters and segments restrict both periods; a filter on the
    // shifted measure restricts the combined rows, selected or not.
    let filtered = presto(&format!(
        r#"{{"measures": ["sales.amount"], "dimensions": ["stores.store_name"], "segments": ["stores.open"],
            "timeDimensions": [{{"dimension": "sales.sold_at", {YEAR_BY_MONTH}}}],
            "filters": [{{"member": "stores.store_name", "operator": "equals", "values": ["central"]}},
                        {{"member": "sales.amount_prior_year", "operator": "gt", "values": ["0"]}}]}}"#
    ));
    assert_eq!(
        filtered
            .sql
            .matches(r#"AND ("stores".store_name = ?) AND ("stores".closed_at IS NULL)"#)
            .count(),
        2,
        "{}",
        filtered.sql
    );
    assert!(
        filtered
            .sql
            .contains(r#"."sales__amount_prior_year" > CAST(? AS DOUBLE))"#),
        "{}",
        filtered.sql
    );
    let aliases: Vec<_> = filtered.columns.iter().map(|c| c.alias.as_str()).collect();
    assert_eq!(
        aliases,
        [
            "stores__store_name",
            "sales__sold_at_month",
            "sales__amount"
        ]
    );
}

// scenario: B1-WIN-001
#[test]
fn window_or_shift_without_its_time_dimension_is_unsupported_by_name() {
    for dialect in [DialectId::Presto, DialectId::Doris] {
        // Named, computed from, or only filtered on: without a time dimension
        // each would silently be a total over all time.
        for (query, measure) in [
            (r#"{"measures": ["sales.amount_7d"]}"#, "sales.amount_7d"),
            (
                r#"{"measures": ["sales.amount_mtd"], "dimensions": ["sales.channel_cd"]}"#,
                "sales.amount_mtd",
            ),
            (
                r#"{"measures": ["sales.amount_prior_year"]}"#,
                "sales.amount_prior_year",
            ),
            (
                r#"{"measures": ["sales.growth"]}"#,
                "sales.amount_prior_year",
            ),
            (
                r#"{"measures": ["sales.amount"],
                    "filters": [{"member": "sales.amount_so_far", "operator": "gt", "values": ["1"]}]}"#,
                "sales.amount_so_far",
            ),
        ] {
            assert_eq!(
                unsupported(dialect, query),
                format!(
                    "Measure `{measure}` rolls or shifts over time, and the query has no time \
                     dimension; without one it would be computed over all time"
                ),
                "{query}"
            );
        }
        // A shift along a named time dimension does nothing along another.
        assert_eq!(
            unsupported(
                dialect,
                r#"{"measures": ["sales.amount_prior_year"],
                    "timeDimensions": [{"dimension": "stores.opened_at", "granularity": "month"}]}"#
            ),
            "Measure `sales.amount_prior_year` shifts along `sales.sold_at`, which is not a time \
             dimension of the query; it would not be shifted"
        );
        // A shift that names none follows the query's.
        assert!(compile_in(
            &context(dialect),
            r#"{"measures": ["sales.amount_next_month"],
                "timeDimensions": [{"dimension": "stores.opened_at", "granularity": "month"}]}"#
        )
        .is_ok());
    }
}

// scenario: B1-PRESTO-001
#[test]
fn presto_refuses_what_its_intervals_and_generated_series_cannot_express() {
    let presto = |query: &str| unsupported(DialectId::Presto, query);
    let week = r#""granularity": "week", "dateRange": ["2026-10-05", "2026-10-25"]"#;
    let quarter = r#""granularity": "quarter", "dateRange": ["2026-01-01", "2026-12-31"]"#;
    // No interval of a week or a quarter: neither a series by one, nor a window
    // or a shift of one.
    for (query, message) in [
        (
            over(r#""sales.amount_7d""#, week),
            "Measure `sales.amount_7d` has a rolling window, and presto cannot generate a time series by week",
        ),
        (
            over(r#""sales.amount_ytd""#, quarter),
            "Measure `sales.amount_ytd` has a rolling window, and presto cannot generate a time series by quarter",
        ),
        (
            over(r#""sales.amount_2w""#, THREE_DAYS),
            "Measure `sales.amount_2w` uses the interval `2 week`, and presto has no interval of that unit",
        ),
        (
            over(r#""sales.amount_prior_quarter""#, YEAR_BY_MONTH),
            "Measure `sales.amount_prior_quarter` uses the interval `1 quarter`, and presto has no interval of that unit",
        ),
    ] {
        assert_eq!(presto(&query), message, "{query}");
        assert!(compile_in(&context(DialectId::Doris), &query).is_ok(), "{query}");
    }
    // The generated series starts where the range does, so the range has to start a period.
    for time in [
        r#""granularity": "month", "dateRange": ["2026-01-15", "2026-12-31"]"#,
        r#""granularity": "year", "dateRange": ["2026-02-01", "2026-12-31"]"#,
        r#""granularity": "day", "dateRange": ["2026-10-01T12:00:00", "2026-10-03"]"#,
    ] {
        let query = over(r#""sales.amount_7d""#, time);
        assert!(
            presto(&query).contains(
                "its time series starts at the start of the date range of `sales.sold_at`"
            ),
            "{time}"
        );
        assert!(
            compile_in(&context(DialectId::Doris), &query).is_ok(),
            "{time}"
        );
    }
    // A range that starts a period, written as a date or as its first instant,
    // and a range no series is built for.
    for time in [
        r#""granularity": "month", "dateRange": ["2026-01-01T00:00:00.000", "2026-12-15"]"#,
        r#""granularity": "year", "dateRange": ["2026-01-01", "2026-06-30"]"#,
        r#""dateRange": ["2026-01-15", "2026-12-31"]"#,
        // Without a range the series runs over the periods the data has.
        r#""granularity": "month""#,
    ] {
        let query = over(r#""sales.amount_7d""#, time);
        assert!(
            compile_in(&context(DialectId::Presto), &query).is_ok(),
            "{time}"
        );
    }
    // A shift builds no series: any range will do.
    assert!(compile_in(
        &context(DialectId::Presto),
        &over(
            r#""sales.amount_prior_year""#,
            r#""granularity": "week", "dateRange": ["2026-10-01", "2026-10-25"]"#
        )
    )
    .is_ok());
}

// scenario: B1-DORIS-002
// scenario: B0-DET-001
#[test]
fn doris_writes_the_periods_out_and_shifts_with_its_own_functions() {
    let queries = [
        over(r#""sales.amount_7d", "sales.amount_mtd""#, THREE_DAYS),
        over(
            r#""sales.amount_2w""#,
            r#""granularity": "week", "dateRange": ["2026-10-01", "2026-10-14"]"#,
        ),
        over(
            r#""sales.amount", "sales.amount_prior_quarter""#,
            r#""granularity": "quarter", "dateRange": ["2026-01-01", "2026-06-30"]"#,
        ),
    ];
    insta::assert_snapshot!(queries
        .each_ref()
        .map(|query| compiled(DialectId::Doris, query))
        .join("\n\n"));
    // The periods are written out, so there is nothing to write without a range.
    assert_eq!(
        unsupported(DialectId::Doris, &over(r#""sales.amount_7d""#, r#""granularity": "day""#)),
        "Measure `sales.amount_7d` has a rolling window, and on doris `sales.sold_at` needs a date \
         range to roll by its granularity"
    );
    // Where both dialects compile, columns and lineage are the same. So are the
    // parameters of a shift; a window's source reaches as far as the series does,
    // and the two dialects end theirs differently.
    for time in [THREE_DAYS, OCTOBER, YEAR_BY_MONTH] {
        for (measures, same_parameters) in [
            (r#""sales.amount", "sales.amount_prior_year""#, true),
            (r#""sales.amount_7d", "sales.amount_prior_year""#, false),
        ] {
            let query = over(measures, time);
            let doris = compile_in(&context(DialectId::Doris), &query).expect("the query compiles");
            let presto = presto(&query);
            assert_eq!(doris.columns, presto.columns, "{time}");
            assert_eq!(doris.used_cubes, presto.used_cubes, "{time}");
            assert_eq!(doris.parameters.len(), presto.parameters.len(), "{time}");
            if same_parameters {
                assert_eq!(doris.parameters, presto.parameters, "{time}");
            }
        }
    }
}
