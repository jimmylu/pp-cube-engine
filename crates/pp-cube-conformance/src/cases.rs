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
    /// two regions. A fourth store without a name has no sales. Four refunds
    /// and four visits: no refund at `uptown`, no visit at `harbor`, and both
    /// at the nameless store.
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

/// A case on the retail fixture in UTC over days of `sales.sold_at`: the first
/// column of every row is the day.
fn daily(
    name: &'static str,
    measures: &[&str],
    range: [&str; 2],
    rows: &[(&str, &[Value])],
) -> Declared {
    let quoted: Vec<_> = measures.iter().map(|m| format!(r#""sales.{m}""#)).collect();
    let query = format!(
        r#"{{"measures":[{}],"timeDimensions":[{{"dimension":"sales.sold_at","granularity":"day","dateRange":["{}","{}"]}}],"order":[["sales.sold_at","asc"]]}}"#,
        quoted.join(","),
        range[0],
        range[1]
    );
    let mut columns = vec!["sales__sold_at_day".to_owned()];
    columns.extend(measures.iter().map(|m| format!("sales__{m}")));
    Declared {
        name,
        group: Group::Matrix,
        fixture: Fixture::Retail,
        query,
        timezone: "UTC",
        security: "",
        columns,
        rows: rows
            .iter()
            .map(|(day, values)| {
                let mut row = vec![midnight(day)];
                row.extend(values.iter().cloned());
                row
            })
            .collect(),
    }
}

/// Rolling windows, period to date and time shifts. The retail sales are one a
/// day from 1 to 6 October: 10, 20, 5, 7, 11 and 99.
fn windows() -> Vec<Declared> {
    vec![
        // Each day with the two before it; the seventh has no sale of its own.
        daily(
            "rolling_trailing",
            &["amount_3d"],
            ["2026-10-01", "2026-10-07"],
            &[
                ("2026-10-01", &[json!(10)]),
                ("2026-10-02", &[json!(30)]),
                ("2026-10-03", &[json!(35)]),
                ("2026-10-04", &[json!(32)]),
                ("2026-10-05", &[json!(23)]),
                ("2026-10-06", &[json!(117)]),
                ("2026-10-07", &[json!(110)]),
            ],
        ),
        // The sales of 1 and 2 October are one order: counted over the window's
        // rows it is one, where adding each day's count would give two.
        daily(
            "rolling_distinct_count",
            &["orders_2d"],
            ["2026-10-02", "2026-10-03"],
            &[("2026-10-02", &[json!(1)]), ("2026-10-03", &[json!(2)])],
        ),
        // The month starts before the range does.
        daily(
            "month_to_date",
            &["amount_mtd"],
            ["2026-10-04", "2026-10-06"],
            &[
                ("2026-10-04", &[json!(42)]),
                ("2026-10-05", &[json!(53)]),
                ("2026-10-06", &[json!(152)]),
            ],
        ),
        // Without a granularity there is one window, ending where the range ends.
        retail(
            "rolling_at_range_end",
            r#"{"measures":["sales.amount_3d"],"timeDimensions":[{"dimension":"sales.sold_at","dateRange":["2026-10-01","2026-10-04"]}]}"#,
            &["sales__amount_3d"],
            json!([[32]]),
        ),
        // A month without sales before the first has nothing so far; one after keeps the total.
        retail(
            "running_total_by_month",
            r#"{"measures":["sales.amount_so_far"],"timeDimensions":[{"dimension":"sales.sold_at","granularity":"month","dateRange":["2026-09-01","2026-11-30"]}],"order":[["sales.sold_at","asc"]]}"#,
            &["sales__sold_at_month", "sales__amount_so_far"],
            json!([
                [midnight("2026-09-01"), null],
                [midnight("2026-10-01"), 152],
                [midnight("2026-11-01"), 152]
            ]),
        ),
        daily(
            "prior_day",
            &["amount", "amount_prior_day"],
            ["2026-10-02", "2026-10-04"],
            &[
                ("2026-10-02", &[json!(20), json!(10)]),
                ("2026-10-03", &[json!(5), json!(20)]),
                ("2026-10-04", &[json!(7), json!(5)]),
            ],
        ),
        daily(
            "change_from_prior_day",
            &["amount_change"],
            ["2026-10-02", "2026-10-04"],
            &[
                ("2026-10-02", &[json!(10)]),
                ("2026-10-03", &[json!(-15)]),
                ("2026-10-04", &[json!(2)]),
            ],
        ),
        // The security filter restricts the rows a window is summed over: C1 sold
        // on the first two days only.
        Declared {
            security: r#"{"filters":[{"member":"sales.channel_cd","operator":"equals","values":["C1"]}]}"#,
            ..daily(
                "security_rolling",
                &["amount_3d"],
                ["2026-10-01", "2026-10-03"],
                &[
                    ("2026-10-01", &[json!(10)]),
                    ("2026-10-02", &[json!(30)]),
                    ("2026-10-03", &[json!(30)]),
                ],
            )
        },
    ]
}

/// Rows as they are, an estimated distinct count, and periods a time dimension
/// declares. The retail sales are one a day from Thursday 1 to Tuesday 6 October.
fn detail() -> Vec<Declared> {
    let by = |granularity: &str| {
        format!(
            r#"{{"measures":["sales.amount"],"timeDimensions":[{{"dimension":"sales.sold_at","granularity":"{granularity}","dateRange":["2026-10-01","2026-10-31"]}}],"order":[["sales.sold_at","asc"]]}}"#
        )
    };
    vec![
        // One row per sale: the sum is the amount, the count is one, and the
        // filtered sum is absent on the sale that is not a toy.
        retail(
            "ungrouped_rows",
            r#"{"ungrouped":true,"dimensions":["sales.sale_id","sales.channel_cd"],"measures":["sales.amount","sales.row_count","sales.toy_amount"],"filters":[{"member":"sales.channel_cd","operator":"equals","values":["C2"]}],"order":[["sales.sale_id","asc"]]}"#,
            &[
                "sales__sale_id",
                "sales__channel_cd",
                "sales__amount",
                "sales__row_count",
                "sales__toy_amount",
            ],
            json!([[3, "C2", 5, 1, null], [4, "C2", 7, 1, 7]]),
        ),
        // Rows of a joined cube beside those of the fact, a page of them.
        retail(
            "ungrouped_joined_page",
            r#"{"ungrouped":true,"dimensions":["sales.sale_id","stores.store_name"],"measures":["sales.amount"],"order":[["sales.sale_id","desc"]],"limit":2,"offset":1}"#,
            &["sales__sale_id", "stores__store_name", "sales__amount"],
            json!([[5, "uptown", 11], [4, "harbor", 7]]),
        ),
        // With so few values the engines' estimates are exact: the two sales of
        // C1 are one order, those of C3 another.
        retail(
            "approximate_distinct_count",
            r#"{"measures":["sales.order_count_approx"],"dimensions":["sales.channel_cd"],"order":[["sales.channel_cd","asc"]]}"#,
            &["sales__channel_cd", "sales__order_count_approx"],
            json!([["C1", 1], ["C2", 2], ["C3", 1]]),
        ),
        retail(
            "granularity_of_two_days",
            &by("two_days"),
            &["sales__sold_at_two_days", "sales__amount"],
            json!([
                [midnight("2026-10-01"), 30],
                [midnight("2026-10-03"), 12],
                [midnight("2026-10-05"), 110]
            ]),
        ),
        // Two quarters from January: October lies in the half that starts in July.
        retail(
            "granularity_of_two_quarters",
            &by("half_year"),
            &["sales__sold_at_half_year", "sales__amount"],
            json!([[midnight("2026-07-01"), 152]]),
        ),
        // A natural week would split these six days at Monday the 5th.
        retail(
            "granularity_with_offset",
            &by("retail_week"),
            &["sales__sold_at_retail_week", "sales__amount"],
            json!([[midnight("2026-10-01"), 152]]),
        ),
    ]
}

/// Queries on a view over sales, stores, regions and receipts.
fn views() -> Vec<Declared> {
    vec![
        retail(
            "view_by_joined_dimension",
            r#"{"measures":["sales_overview.amount"],"dimensions":["sales_overview.stores_store_name"],"order":[["sales_overview.stores_store_name","asc"]]}"#,
            &[
                "sales_overview__stores_store_name",
                "sales_overview__amount",
            ],
            json!([["central", 30], ["harbor", 12], ["uptown", 110]]),
        ),
        // The region is two joins away; the filter is on a member of the fact.
        retail(
            "view_two_hops_filtered",
            r#"{"measures":["sales_overview.amount","sales_overview.row_count"],"dimensions":["sales_overview.region_name"],"filters":[{"member":"sales_overview.channel_cd","operator":"equals","values":["C1","C2"]}],"order":[["sales_overview.region_name","asc"]]}"#,
            &[
                "sales_overview__region_name",
                "sales_overview__amount",
                "sales_overview__row_count",
            ],
            json!([["north", 30, 2], ["south", 12, 2]]),
        ),
        // Each channel sells at one store, whose area counts once for its two sales.
        retail(
            "view_sum_over_repeated_rows",
            r#"{"measures":["sales_overview.amount","sales_overview.stores_floor_area"],"dimensions":["sales_overview.channel_cd"],"order":[["sales_overview.channel_cd","asc"]]}"#,
            &[
                "sales_overview__channel_cd",
                "sales_overview__amount",
                "sales_overview__stores_floor_area",
            ],
            json!([["C1", 30, 100], ["C2", 12, 200], ["C3", 110, 50]]),
        ),
        // A formula of the fact beside a measure of the cube joined one_to_one.
        retail(
            "view_formula_and_one_to_one",
            r#"{"measures":["sales_overview.amount_per_order","sales_overview.tax"],"dimensions":["sales_overview.channel_cd"],"order":[["sales_overview.channel_cd","asc"]]}"#,
            &[
                "sales_overview__channel_cd",
                "sales_overview__amount_per_order",
                "sales_overview__tax",
            ],
            json!([["C1", 30, 3], ["C2", 6, 5], ["C3", 110, 12]]),
        ),
        // The security filter names a view member and restricts the cube behind it.
        Declared {
            security: r#"{"filters":[{"member":"sales_overview.region_name","operator":"equals","values":["north"]}]}"#,
            ..retail(
                "security_view",
                r#"{"measures":["sales_overview.amount"],"dimensions":["sales_overview.stores_store_name"],"order":[["sales_overview.stores_store_name","asc"]]}"#,
                &[
                    "sales_overview__stores_store_name",
                    "sales_overview__amount",
                ],
                json!([["central", 30], ["uptown", 110]]),
            )
        },
    ]
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
        // Days are Tokyo's: 1 October holds the sale of 20, and its window reaches
        // back to the sale of 10 in the last millisecond of September.
        Declared {
            group: Group::Temporal,
            fixture: Fixture::TokyoOctober,
            timezone: "Asia/Tokyo",
            ..daily(
                "rolling_tokyo",
                &["amount_2d"],
                ["2026-10-01", "2026-10-02"],
                &[("2026-10-01", &[json!(30)]), ("2026-10-02", &[json!(20)])],
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
        // Measures of a cube joined one_to_one: the sale without a receipt adds nothing,
        // and the receipt no sale has is not counted.
        retail(
            "joined_one_to_one_measures",
            r#"{"measures":["receipts.tax","receipts.receipt_count","receipts.tax_per_receipt"],"dimensions":["sales.channel_cd"],"order":[["sales.channel_cd","asc"]]}"#,
            &[
                "sales__channel_cd",
                "receipts__tax",
                "receipts__receipt_count",
                "receipts__tax_per_receipt",
            ],
            json!([["C1", 3, 2, 1.5], ["C2", 5, 1, 5], ["C3", 12, 2, 6]]),
        ),
        // Counts of a cube joined many_to_one count distinct keys, not sales: three
        // sales of each category reach two stores.
        retail(
            "joined_many_to_one_counts",
            r#"{"measures":["stores.store_count","stores.name_count"],"dimensions":["sales.category_cd"],"order":[["sales.category_cd","asc"]]}"#,
            &[
                "sales__category_cd",
                "stores__store_count",
                "stores__name_count",
            ],
            json!([["BOOK", 2, 2], ["TOY", 2, 2]]),
        ),
        // The join to promotions reads the region, reached through the store. Without
        // the region in the condition, the south sale would count under both names.
        retail(
            "join_reading_a_third_cube",
            r#"{"measures":["sales.amount"],"dimensions":["promos.promo_name"],"filters":[{"member":"promos.promo_name","operator":"set"}],"order":[["promos.promo_name","asc"]]}"#,
            &["promos__promo_name", "sales__amount"],
            json!([["spring-north", 30], ["spring-south", 5]]),
        ),
        // Each fact is totalled by store on its own: `uptown` has no refund. The
        // filter drops the nameless store from both facts.
        retail(
            "multi_fact_by_dimension",
            r#"{"measures":["sales.amount","refunds.amount","refunds.refund_count"],"dimensions":["stores.store_name"],"filters":[{"member":"stores.store_name","operator":"set"}],"order":[["stores.store_name","asc"]]}"#,
            &[
                "stores__store_name",
                "sales__amount",
                "refunds__amount",
                "refunds__refund_count",
            ],
            json!([
                ["central", 30, 10, 2],
                ["harbor", 12, 5, 1],
                ["uptown", 110, null, null]
            ]),
        ),
        // Three facts. The nameless store has refunds and visits, which meet in one
        // row only if NULL keys are matched. The measure filter drops `harbor`,
        // which has no visit, from the combined rows.
        retail(
            "multi_fact_null_key",
            r#"{"measures":["sales.amount","refunds.amount","visits.visit_count"],"dimensions":["stores.store_name"],"filters":[{"member":"visits.visit_count","operator":"gt","values":["0"]}],"order":[["visits.visit_count","desc"],["sales.amount","desc"]]}"#,
            &[
                "stores__store_name",
                "sales__amount",
                "refunds__amount",
                "visits__visit_count",
            ],
            json!([
                [null, null, 8, 2],
                ["uptown", 110, null, 1],
                ["central", 30, 10, 1]
            ]),
        ),
        // Without a dimension each fact is one row, and they stand side by side.
        retail(
            "multi_fact_totals",
            r#"{"measures":["sales.amount","refunds.amount"]}"#,
            &["sales__amount", "refunds__amount"],
            json!([[152, 23]]),
        ),
        // The page is taken from the combined rows: 110, then 30, then 12.
        retail(
            "multi_fact_page",
            r#"{"measures":["sales.amount","refunds.amount"],"dimensions":["stores.store_name"],"filters":[{"member":"stores.store_name","operator":"set"}],"order":[["sales.amount","desc"]],"limit":1,"offset":1}"#,
            &["stores__store_name", "sales__amount", "refunds__amount"],
            json!([["central", 30, 10]]),
        ),
        // Three sales of each category reach two stores; each store's area counts
        // once. Summed over the joined rows the areas would be 300 and 400.
        retail(
            "sum_over_repeated_rows",
            r#"{"measures":["sales.amount","stores.floor_area"],"dimensions":["sales.category_cd"],"order":[["sales.category_cd","asc"]]}"#,
            &["sales__category_cd", "sales__amount", "stores__floor_area"],
            json!([["BOOK", 115, 250], ["TOY", 37, 300]]),
        ),
        // A one_to_one join repeats no row, so both cubes aggregate in one SELECT.
        retail(
            "two_cubes_in_one_select",
            r#"{"measures":["sales.amount","receipts.tax"],"dimensions":["sales.channel_cd"],"order":[["sales.channel_cd","asc"]]}"#,
            &["sales__channel_cd", "sales__amount", "receipts__tax"],
            json!([["C1", 30, 3], ["C2", 12, 5], ["C3", 110, 12]]),
        ),
        // The security filter restricts every fact: `harbor` and the nameless
        // store, both in the south, lose their sales and their refunds alike.
        Declared {
            security: r#"{"filters":[{"member":"regions.region_name","operator":"equals","values":["north"]}]}"#,
            ..retail(
                "security_multi_fact",
                r#"{"measures":["sales.amount","refunds.amount"],"dimensions":["stores.store_name"],"order":[["stores.store_name","asc"]]}"#,
                &["stores__store_name", "sales__amount", "refunds__amount"],
                json!([["central", 30, 10], ["uptown", 110, null]]),
            )
        },
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
    cases.extend(windows());
    cases.extend(detail());
    cases.extend(views());
    cases
}
