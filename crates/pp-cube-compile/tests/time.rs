//! Time dimensions, date ranges, date operators and the timezone, compiled
//! for Presto.
//!
//! The snapshots are the SQL the pinned planner emits with the Presto
//! templates, followed by the parameters in placeholder order. Nobody has
//! executed them; execution belongs to tiers T1 and T2.

mod support;
use support::compile;

use std::sync::Arc;

use pp_cube_compile::{CompileContext, CompileError, Compiled, DialectId, Parameter};
use pp_cube_model::{load_query, load_yaml};

/// Compiles `sales.amount` with `rest` added to the query, with `timezone`
/// as the one of the compile context.
fn compile_in(timezone: &str, rest: &str) -> Result<Compiled, CompileError> {
    let model = Arc::new(load_yaml(include_str!("fixtures/sales.yaml")).expect("the model loads"));
    let query = format!(r#"{{"measures": ["sales.amount"], {rest}}}"#);
    let query = load_query(&query).expect("the query loads");
    let context = CompileContext {
        request_id: String::new(),
        dialect: DialectId::Presto,
        timezone: timezone.to_owned(),
        security_filters: Vec::new(),
    };
    compile(&model, &query, &context)
}

/// SQL and parameters, as one text to snapshot.
fn compiled(timezone: &str, rest: &str) -> String {
    let compiled = compile_in(timezone, rest).expect("the query compiles");
    format!("{}\n-- parameters: {:?}", compiled.sql, compiled.parameters)
}

fn query_failure(timezone: &str, rest: &str) -> String {
    match compile_in(timezone, rest) {
        Err(CompileError::Query(message)) => message,
        other => panic!("expected a query failure, got {other:?}"),
    }
}

/// What reading a query with `time_dimension` as its only one fails with.
fn load_failure(time_dimension: &str) -> String {
    let query = format!(r#"{{"timeDimensions": [{time_dimension}]}}"#);
    load_query(&query)
        .expect_err("the query is not read")
        .message
}

// scenario: B0-PRESTO-001
// scenario: B0-DET-001
#[test]
fn all_granularities_in_tokyo() {
    for granularity in ["day", "week", "month", "quarter", "year"] {
        let query = format!(
            r#""timeDimensions": [{{
                 "dimension": "sales.sold_at",
                 "granularity": "{granularity}",
                 "dateRange": ["2026-10-01", "2026-10-31"]
               }}]"#
        );
        insta::assert_snapshot!(
            format!("{granularity}_granularity_in_tokyo"),
            compiled("Asia/Tokyo", &query)
        );
    }
}

// scenario: B0-QUERY-001
// scenario: B0-DET-001
// scenario: B0-PRESTO-015
#[test]
fn each_granularity_truncates_the_time_dimension_in_the_timezone() {
    insta::assert_snapshot!(compiled(
        "UTC",
        r#""timeDimensions": [
             {"dimension": "sales.sold_at", "granularity": "day"},
             {"dimension": "sales.sold_at", "granularity": "week"},
             {"dimension": "sales.sold_at", "granularity": "month"},
             {"dimension": "sales.sold_at", "granularity": "quarter"},
             {"dimension": "sales.sold_at", "granularity": "year"}
           ]"#
    ));
}

// scenario: B0-QUERY-001
// scenario: B0-DET-001
// scenario: B0-PRESTO-013
#[test]
fn each_date_operator_compares_with_the_bounds_of_its_dates() {
    insta::assert_snapshot!(compiled(
        "UTC",
        r#""filters": [
             {"member": "sales.sold_at", "operator": "inDateRange", "values": ["2026-01-01", "2026-12-31"]},
             {"member": "sales.sold_at", "operator": "notInDateRange", "values": ["2026-07-01", "2026-07-31"]},
             {"member": "sales.sold_at", "operator": "beforeDate", "values": ["2026-12-25"]},
             {"member": "sales.sold_at", "operator": "beforeOrOnDate", "values": ["2026-12-24"]},
             {"member": "sales.sold_at", "operator": "afterDate", "values": ["2026-01-05"]},
             {"member": "sales.sold_at", "operator": "afterOrOnDate", "values": ["2026-01-06T08:30:00"]}
           ]"#
    ));
}

// scenario: B0-QUERY-003
// scenario: B0-DET-001
// scenario: B0-PRESTO-014
#[test]
fn october_is_closed_at_its_first_and_its_last_millisecond() {
    insta::assert_snapshot!(compiled(
        "UTC",
        r#""timezone": "UTC",
           "timeDimensions": [
             {"dimension": "sales.sold_at", "dateRange": ["2026-10-01", "2026-10-31"]}
           ]"#
    ));
}

// scenario: B0-QUERY-010
// scenario: B0-DET-001
// scenario: B0-PRESTO-016
#[test]
fn query_timezone_wins_over_the_one_of_the_context() {
    let october_by_month = r#""timeDimensions": [{
        "dimension": "sales.sold_at",
        "granularity": "month",
        "dateRange": ["2026-10-01", "2026-10-31"]
    }]"#;
    let context_only = compiled("Asia/Tokyo", october_by_month);
    let query_too = compiled(
        "Asia/Tokyo",
        &format!(r#""timezone": "Europe/Berlin", {october_by_month}"#),
    );

    insta::assert_snapshot!(format!(
        "-- context Asia/Tokyo\n{context_only}\n\n-- context Asia/Tokyo, query Europe/Berlin\n{query_too}"
    ));
}

// scenario: B0-DET-001
#[test]
fn bounds_are_moved_to_utc_the_way_cube_moves_them() {
    let bounds = |timezone: &str, from: &str, to: &str| {
        let range = format!(
            r#""timeDimensions": [{{"dimension": "sales.sold_at", "dateRange": ["{from}", "{to}"]}}]"#
        );
        compile_in(timezone, &range)
            .expect("the query compiles")
            .parameters
    };
    let text =
        |from: &str, to: &str| vec![Parameter::Text(from.into()), Parameter::Text(to.into())];

    assert_eq!(
        [
            // Nine hours ahead of UTC all year.
            bounds("Asia/Tokyo", "2026-10-01", "2026-10-31"),
            // New York's clocks go back an hour on 2026-11-01 at 06:00 UTC.
            // 03:00 local is 08:00 UTC; Cube looks the offset up at 03:00 UTC,
            // before the change, and so does the kernel.
            bounds(
                "America/New_York",
                "2026-11-01T03:00:00",
                "2026-11-01T07:00:00"
            ),
        ],
        [
            text("2026-09-30T15:00:00.000Z", "2026-10-31T14:59:59.999Z"),
            text("2026-11-01T07:00:00.000Z", "2026-11-01T12:00:00.000Z"),
        ]
    );
}

// scenario: B0-QUERY-004
#[test]
fn relative_range_is_refused_instead_of_being_resolved_from_the_clock() {
    let two_dates = "a list of two dates, the start and the end of the range";

    let relative = load_failure(r#"{"dimension": "sales.sold_at", "dateRange": "last month"}"#);
    let relative_end =
        load_failure(r#"{"dimension": "sales.sold_at", "dateRange": ["2026-10-01", "now"]}"#);

    assert!(relative.contains(two_dates), "{relative}");
    assert!(
        relative_end.contains("relative dates are resolved by the caller"),
        "{relative_end}"
    );
}

// scenario: B0-QUERY-008
#[test]
fn unknown_granularity_is_refused_naming_the_rule() {
    // Not a name at all: refused where the query is read.
    let message = load_failure(r#"{"dimension": "sales.sold_at", "granularity": "Fort Night"}"#);
    assert!(
        message.contains("Fort Night")
            && message
                .contains("predefined granularities are `day`, `week`, `month`, `quarter`, `year`")
            && ["`second`", "`minute`", "`hour`"]
                .iter()
                .all(|name| !message.contains(name)),
        "{message}"
    );
    // A name the time dimension does not declare: refused against the model.
    assert_eq!(
        query_failure(
            "UTC",
            r#""timeDimensions": [{"dimension": "sales.sold_at", "granularity": "fortnight"}]"#
        ),
        "Granularity `fortnight` is not one `sales.sold_at` declares"
    );
}

// scenario: B0-QUERY-009
#[test]
fn recognized_subday_granularities_are_unsupported() {
    for granularity in ["second", "minute", "hour"] {
        let error = compile_in(
            "UTC",
            &format!(
                r#""timeDimensions": [{{"dimension": "sales.sold_at", "granularity": "{granularity}"}}]"#
            ),
        )
        .expect_err("sub-day granularity is outside B0");
        assert!(matches!(error, CompileError::Unsupported(_)), "{error:?}");
    }
}

#[test]
fn time_dimension_must_be_one_and_ask_for_something() {
    let failure = |time_dimension: &str| {
        query_failure("UTC", &format!(r#""timeDimensions": [{time_dimension}]"#))
    };

    assert_eq!(
        [
            failure(r#"{"dimension": "sales.qty", "granularity": "day"}"#),
            failure(r#"{"dimension": "sales.amount", "granularity": "day"}"#),
            failure(r#"{"dimension": "sales.missing", "granularity": "day"}"#),
            failure(r#"{"dimension": "sales.sold_at"}"#),
        ],
        [
            "`sales.qty` is a number dimension; `timeDimensions` takes time dimensions",
            "`sales.amount` is a measure; `timeDimensions` takes time dimensions",
            "`sales.missing` is not a dimension or a measure of the model",
            "Time dimension `sales.sold_at` asks for neither a granularity nor a date range",
        ]
    );
}

#[test]
fn date_operator_takes_a_time_dimension_and_its_number_of_dates() {
    let failure = |member: &str, operator: &str, values: &str| {
        query_failure(
            "UTC",
            &format!(
                r#""filters": [{{"member": "sales.{member}", "operator": "{operator}", "values": {values}}}]"#
            ),
        )
    };

    assert_eq!(
        [
            failure(
                "channel_cd",
                "inDateRange",
                r#"["2026-10-01", "2026-10-31"]"#
            ),
            failure("amount", "beforeDate", r#"["2026-10-01"]"#),
            failure("sold_at", "inDateRange", r#"["2026-10-01"]"#),
            failure("sold_at", "afterDate", r#"["2026-10-01", "2026-10-31"]"#),
            failure("sold_at", "beforeDate", "[]"),
            failure("sold_at", "afterOrOnDate", r#"["yesterday"]"#),
        ],
        [
            "Operator `inDateRange` does not fit `sales.channel_cd`, a string dimension",
            "Operator `beforeDate` does not fit `sales.amount`, a measure",
            "Operator `inDateRange` on `sales.sold_at` takes two dates",
            "Operator `afterDate` on `sales.sold_at` takes one date",
            "Operator `beforeDate` on `sales.sold_at` takes one date",
            "Operator `afterOrOnDate` on `sales.sold_at`: `yesterday` is not a date: write \
             `YYYY-MM-DD` or `YYYY-MM-DDTHH:mm:ss.SSS`, without a zone; relative dates are \
             resolved by the caller",
        ]
    );
}

#[test]
fn unknown_timezone_is_refused_wherever_it_is_named() {
    let by_day = r#""timeDimensions": [{"dimension": "sales.sold_at", "granularity": "day"}]"#;

    assert_eq!(
        [
            query_failure("Mars/Olympus", by_day),
            query_failure("UTC", &format!(r#""timezone": "+09:00", {by_day}"#)),
        ],
        [
            "Timezone `Mars/Olympus` is not known",
            "Timezone `+09:00` is not known",
        ]
    );
}

// scenario: B0-DET-001
#[test]
fn order_names_a_time_dimension_by_its_member_where_it_is_grouped_by() {
    let ordered = |time_dimension: &str| {
        compile_in(
            "UTC",
            &format!(
                r#""timeDimensions": [{{"dimension": "sales.sold_at", {time_dimension}}}],
                   "order": {{"sales.amount": "asc", "sales.sold_at": "desc"}}"#
            ),
        )
    };

    let by_month = ordered(r#""granularity": "month""#).expect("the query compiles");
    let range_only = ordered(r#""dateRange": ["2026-10-01", "2026-10-31"]"#);

    assert!(
        by_month.sql.ends_with("ORDER BY  2  ASC,  1  DESC"),
        "{}",
        by_month.sql
    );
    assert_eq!(
        range_only,
        Err(CompileError::Query(
            "Order member `sales.sold_at` is not among the query's measures and dimensions"
                .to_owned()
        ))
    );
}
