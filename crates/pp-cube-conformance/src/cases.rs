//! Execution cases shared by every dialect.
//!
//! A case states a query and the rows it must return on a named fixture. Each
//! dialect supplies the statements that build that fixture on its engine and the
//! scenario its cases support, so all engines answer to the same expectations.

use serde_json::{json, Value};

use crate::midnight;

/// The data a case runs on. Every dialect loads the same rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fixture {
    /// Four sales at UTC instants one millisecond either side of the Tokyo
    /// October boundaries: 10, 20, 7 and 100.
    TokyoOctober,
    /// New York leaves daylight time on 2026-11-01, a 25-hour local day:
    /// 3, 5 and 7 inside it, 100 after it.
    NewYorkDst,
    /// Six sales: a NULL region, a region made of pattern characters and its
    /// decoy `axxb`, one invalid row, two rows of one order, three stores in
    /// two regions.
    Retail,
}

/// What a case is evidence for; a dialect maps each group to its scenario id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Temporal,
    Matrix,
    EmptyPage,
}

/// One case before a dialect gives it a fixture and a scenario.
#[derive(Debug, Clone)]
pub struct Declared {
    pub name: &'static str,
    pub group: Group,
    pub fixture: Fixture,
    pub query: String,
    pub timezone: &'static str,
    pub security: &'static str,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
}

fn sales(rest: &str) -> String {
    format!(r#"{{"measures":["sales.amount"],{rest}}}"#)
}

fn grouped(unit: &str, range: &str) -> String {
    sales(&format!(
        r#""timeDimensions":[{{"dimension":"sales.sold_at","granularity":"{unit}","dateRange":{range}}}]"#
    ))
}

fn bucket(name: &'static str, unit: &str, rows: &[(&str, u32)]) -> Declared {
    Declared {
        name,
        group: Group::Temporal,
        fixture: Fixture::TokyoOctober,
        query: grouped(unit, r#"["2026-10-01","2026-10-31"]"#),
        timezone: "Asia/Tokyo",
        security: "",
        columns: vec![format!("sales__sold_at_{unit}"), "sales__amount".into()],
        rows: rows
            .iter()
            .map(|(day, amount)| vec![midnight(day), json!(amount)])
            .collect(),
    }
}

fn total(name: &'static str, filters: &str, amount: u32) -> Declared {
    Declared {
        name,
        group: Group::Temporal,
        fixture: Fixture::TokyoOctober,
        query: sales(&format!(r#""filters":[{filters}]"#)),
        timezone: "Asia/Tokyo",
        security: "",
        columns: vec!["sales__amount".into()],
        rows: vec![vec![Value::from(amount)]],
    }
}

fn date_filter(name: &'static str, values: &str, amount: u32) -> Declared {
    let filter = format!(r#"{{"member":"sales.sold_at","operator":"{name}","values":{values}}}"#);
    total(name, &filter, amount)
}

/// A case on the retail fixture in UTC: `query` JSON, expected columns and rows.
fn retail(name: &'static str, query: &str, columns: &[&str], rows: Value) -> Declared {
    let Value::Array(rows) = rows else {
        unreachable!("rows are written as an array")
    };
    Declared {
        name,
        group: Group::Matrix,
        fixture: Fixture::Retail,
        query: query.into(),
        timezone: "UTC",
        security: "",
        columns: columns.iter().map(|column| (*column).into()).collect(),
        rows: rows
            .into_iter()
            .map(|row| row.as_array().cloned().unwrap_or_default())
            .collect(),
    }
}

/// The total of `sales.amount` over the retail rows that pass `filters`.
fn passing(name: &'static str, filters: &str, amount: u32) -> Declared {
    retail(
        name,
        &sales(&format!(r#""filters":[{filters}]"#)),
        &["sales__amount"],
        json!([[amount]]),
    )
}

fn region(name: &'static str, operator: &str, amount: u32) -> Declared {
    // Upper case, so a match also proves case-insensitive comparison.
    let filter =
        format!(r#"{{"member":"sales.region_cd","operator":"{operator}","values":["A%_\\B"]}}"#);
    passing(name, &filter, amount)
}

fn quantity(name: &'static str, operator: &str, amount: u32) -> Declared {
    let filter = format!(r#"{{"member":"sales.qty","operator":"{operator}","values":["3"]}}"#);
    passing(name, &filter, amount)
}

fn temporal() -> Vec<Declared> {
    let range = r#"["2026-10-01","2026-10-31"]"#;
    let day = r#"["2026-10-01"]"#;
    vec![
        bucket("day", "day", &[("2026-10-01", 20), ("2026-10-31", 7)]),
        bucket("week", "week", &[("2026-09-28", 20), ("2026-10-26", 7)]),
        bucket("month", "month", &[("2026-10-01", 27)]),
        bucket("quarter", "quarter", &[("2026-10-01", 27)]),
        bucket("year", "year", &[("2026-01-01", 27)]),
        date_filter("inDateRange", range, 27),
        date_filter("notInDateRange", range, 110),
        date_filter("beforeDate", day, 10),
        date_filter("beforeOrOnDate", day, 30),
        date_filter("afterDate", day, 107),
        date_filter("afterOrOnDate", day, 127),
        // A user OR that admits every row must not widen the security range.
        Declared {
            security: r#"{"filters":[{"member":"sales.sold_at","operator":"inDateRange","values":["2026-10-01","2026-10-31"]}]}"#,
            ..total(
                "security",
                r#"{"or":[{"member":"sales.sold_at","operator":"beforeDate","values":["2026-10-01"]},{"member":"sales.sold_at","operator":"afterOrOnDate","values":["2026-10-01"]}]}"#,
                27,
            )
        },
        Declared {
            name: "dst",
            fixture: Fixture::NewYorkDst,
            query: grouped("day", r#"["2026-11-01","2026-11-01"]"#),
            timezone: "America/New_York",
            rows: vec![vec![midnight("2026-11-01"), json!(15)]],
            ..bucket("dst", "day", &[])
        },
    ]
}

fn matrix() -> Vec<Declared> {
    let by_channel = r#""dimensions":["sales.channel_cd"],"order":[["sales.channel_cd","asc"]]"#;
    vec![
        retail(
            "aggregates",
            &format!(
                r#"{{"measures":["sales.row_count","sales.order_count","sales.amount","sales.avg_amount","sales.min_amount","sales.max_amount"],{by_channel}}}"#
            ),
            &[
                "sales__channel_cd",
                "sales__row_count",
                "sales__order_count",
                "sales__amount",
                "sales__avg_amount",
                "sales__min_amount",
                "sales__max_amount",
            ],
            json!([
                ["C1", 2, 1, 30, 15, 10, 20],
                ["C2", 2, 2, 12, 6, 5, 7],
                ["C3", 2, 1, 110, 55, 11, 99]
            ]),
        ),
        // The segment keeps the three TOY rows; the ratio divides after aggregating.
        retail(
            "filtered_measure_ratio_segment",
            &format!(
                r#"{{"measures":["sales.toy_amount","sales.amount_per_order"],"segments":["sales.toys"],{by_channel}}}"#
            ),
            &[
                "sales__channel_cd",
                "sales__toy_amount",
                "sales__amount_per_order",
            ],
            json!([["C1", 30, 30], ["C2", 7, 7]]),
        ),
        passing(
            "equals_many",
            r#"{"member":"sales.channel_cd","operator":"equals","values":["C1","C3"]}"#,
            140,
        ),
        // Cube's notEquals keeps the row whose region is NULL.
        passing(
            "notEquals",
            r#"{"member":"sales.region_cd","operator":"notEquals","values":["north"]}"#,
            122,
        ),
        passing(
            "boolean_equals",
            r#"{"member":"sales.is_valid","operator":"equals","values":["true"]}"#,
            145,
        ),
        passing(
            "set",
            r#"{"member":"sales.region_cd","operator":"set"}"#,
            145,
        ),
        passing(
            "notSet",
            r#"{"member":"sales.region_cd","operator":"notSet"}"#,
            7,
        ),
        quantity("gt", "gt", 117),
        quantity("gte", "gte", 122),
        quantity("lt", "lt", 30),
        quantity("lte", "lte", 35),
        // Only `a%_\b` matches; `axxb` would match if a wildcard leaked. Negations keep NULL.
        region("contains", "contains", 11),
        region("notContains", "notContains", 141),
        region("startsWith", "startsWith", 11),
        region("notStartsWith", "notStartsWith", 141),
        region("endsWith", "endsWith", 11),
        region("notEndsWith", "notEndsWith", 141),
        // Measure filters restrict groups: C2 totals 12, C3 totals 110.
        retail(
            "measure_filters",
            &format!(
                r#"{{"measures":["sales.amount"],{by_channel},"filters":[{{"member":"sales.amount","operator":"gt","values":["20"]}},{{"member":"sales.amount","operator":"notEquals","values":["110"]}}]}}"#
            ),
            &["sales__channel_cd", "sales__amount"],
            json!([["C1", 30]]),
        ),
        // BOOK and ((C3 and qty > 5) or C2): sales 6 and 3.
        passing(
            "nested_groups",
            r#"{"member":"sales.category_cd","operator":"equals","values":["BOOK"]},{"or":[{"and":[{"member":"sales.channel_cd","operator":"equals","values":["C3"]},{"member":"sales.qty","operator":"gt","values":["5"]}]},{"member":"sales.channel_cd","operator":"equals","values":["C2"]}]}"#,
            104,
        ),
        // Four groups ordered C3/BOOK, C2/BOOK, C2/TOY, C1/TOY; the page is the middle two.
        retail(
            "order_limit_offset",
            r#"{"measures":["sales.amount"],"dimensions":["sales.channel_cd","sales.category_cd"],"order":[["sales.channel_cd","desc"],["sales.category_cd","asc"]],"limit":2,"offset":1}"#,
            &["sales__channel_cd", "sales__category_cd", "sales__amount"],
            json!([["C2", "BOOK", 5], ["C2", "TOY", 7]]),
        ),
        // The same four groups; a page that starts after the last one is empty.
        Declared {
            group: Group::EmptyPage,
            ..retail(
                "page_past_last_group",
                r#"{"measures":["sales.amount"],"dimensions":["sales.channel_cd","sales.category_cd"],"order":[["sales.channel_cd","desc"],["sales.category_cd","asc"]],"limit":2,"offset":4}"#,
                &["sales__channel_cd", "sales__category_cd", "sales__amount"],
                json!([]),
            )
        },
        retail(
            "plain_range",
            &sales(
                r#""timeDimensions":[{"dimension":"sales.sold_at","dateRange":["2026-10-02","2026-10-04"]}]"#,
            ),
            &["sales__amount"],
            json!([[32]]),
        ),
        // The context zone alone would select the sale of 2 October instead.
        Declared {
            timezone: "America/New_York",
            ..retail(
                "query_timezone",
                &sales(
                    r#""timezone":"UTC","timeDimensions":[{"dimension":"sales.sold_at","granularity":"day","dateRange":["2026-10-01","2026-10-01"]}]"#,
                ),
                &["sales__sold_at_day", "sales__amount"],
                json!([[midnight("2026-10-01"), 10]]),
            )
        },
        // The cube's own source drops the invalid sale of C2.
        retail(
            "filtered_source",
            r#"{"measures":["valid_sales.amount"],"dimensions":["valid_sales.channel_cd"],"order":[["valid_sales.channel_cd","asc"]]}"#,
            &["valid_sales__channel_cd", "valid_sales__amount"],
            json!([["C1", 30], ["C2", 5], ["C3", 110]]),
        ),
        retail(
            "joined_dimension",
            r#"{"measures":["sales.amount"],"dimensions":["stores.store_name"],"filters":[{"member":"stores.store_name","operator":"equals","values":["central","uptown"]}],"order":[["stores.store_name","asc"]]}"#,
            &["stores__store_name", "sales__amount"],
            json!([["central", 30], ["uptown", 110]]),
        ),
        // Only the security filter needs the two joins to the region table.
        Declared {
            security: r#"{"filters":[{"member":"regions.region_name","operator":"equals","values":["north"]}]}"#,
            ..passing(
                "security_two_hop_join",
                r#"{"member":"sales.is_valid","operator":"equals","values":["true"]}"#,
                140,
            )
        },
        // The user OR admits four sales; the security pattern admits only `a%_\b`.
        Declared {
            security: r#"{"filters":[{"member":"sales.region_cd","operator":"startsWith","values":["a%"]}]}"#,
            ..passing(
                "security_pattern",
                r#"{"or":[{"member":"sales.region_cd","operator":"contains","values":["north"]},{"member":"sales.channel_cd","operator":"equals","values":["C3"]}]}"#,
                11,
            )
        },
    ]
}

pub fn declared() -> Vec<Declared> {
    let mut cases = temporal();
    cases.extend(matrix());
    cases
}
