//! Doris execution cases.
//!
//! Every fixture creates the same tables when absent, empties them and loads
//! its own rows, so a tier can switch fixtures on one engine.

use serde_json::{json, Value};

use crate::{midnight, Case};

pub const MODEL: &str = include_str!("../fixtures/sales.yaml");

const TEMPORAL: &str = "B0-DORIS-010";
const MATRIX: &str = "B0-DORIS-002";

const DATABASE: &str = "CREATE DATABASE IF NOT EXISTS dw";
const SALES: &str =
    "CREATE TABLE IF NOT EXISTS dw.sales_dtl (sale_id INT, channel_cd VARCHAR(16), \
     category_cd VARCHAR(16), region_cd VARCHAR(32), store_cd VARCHAR(16), is_valid TINYINT, \
     qty INT, order_no INT, amount DOUBLE, sold_at DATETIMEV2(3)) DUPLICATE KEY(sale_id) \
     DISTRIBUTED BY HASH(sale_id) BUCKETS 1 PROPERTIES ('replication_num'='1')";
const STORES: &str = "CREATE TABLE IF NOT EXISTS dw.dim_store (store_cd VARCHAR(16), \
     store_name VARCHAR(32), region_cd VARCHAR(16)) DUPLICATE KEY(store_cd) \
     DISTRIBUTED BY HASH(store_cd) BUCKETS 1 PROPERTIES ('replication_num'='1')";
const REGIONS: &str = "CREATE TABLE IF NOT EXISTS dw.dim_region (region_cd VARCHAR(16), \
     region_name VARCHAR(32)) DUPLICATE KEY(region_cd) \
     DISTRIBUTED BY HASH(region_cd) BUCKETS 1 PROPERTIES ('replication_num'='1')";
const NO_SALES: &str = "TRUNCATE TABLE dw.sales_dtl";
const NO_STORES: &str = "TRUNCATE TABLE dw.dim_store";
const NO_REGIONS: &str = "TRUNCATE TABLE dw.dim_region";

/// UTC instants one millisecond either side of the Tokyo October boundaries.
const TOKYO_OCTOBER: &[&str] = &[
    DATABASE,
    SALES,
    STORES,
    REGIONS,
    NO_SALES,
    NO_STORES,
    NO_REGIONS,
    "INSERT INTO dw.sales_dtl (sale_id, sold_at, amount) VALUES \
     (1,'2026-09-30 14:59:59.999',10),(2,'2026-09-30 15:00:00.000',20),\
     (3,'2026-10-31 14:59:59.999',7),(4,'2026-10-31 15:00:00.000',100)",
];

/// New York leaves daylight time on 2026-11-01, a 25-hour local day.
const NEW_YORK_DST: &[&str] = &[
    DATABASE,
    SALES,
    STORES,
    REGIONS,
    NO_SALES,
    NO_STORES,
    NO_REGIONS,
    "INSERT INTO dw.sales_dtl (sale_id, sold_at, amount) VALUES \
     (1,'2026-11-01 04:00:00',3),(2,'2026-11-01 05:30:00',5),\
     (3,'2026-11-01 06:30:00',7),(4,'2026-11-02 05:00:00',100)",
];

/// Six sales: a NULL region, a region made of pattern characters and its
/// decoy `axxb`, one invalid row, two rows of one order, three stores in two regions.
const RETAIL: &[&str] = &[
    DATABASE,
    SALES,
    STORES,
    REGIONS,
    NO_SALES,
    NO_STORES,
    NO_REGIONS,
    "INSERT INTO dw.sales_dtl VALUES \
     (1,'C1','TOY','north','s1',1,1,100,10,'2026-10-01 00:00:00'),\
     (2,'C1','TOY','north','s1',1,2,100,20,'2026-10-02 00:00:00'),\
     (3,'C2','BOOK','south','s2',1,3,101,5,'2026-10-03 00:00:00'),\
     (4,'C2','TOY',NULL,'s2',0,4,102,7,'2026-10-04 00:00:00'),\
     (5,'C3','BOOK','a%_\\\\b','s3',1,5,103,11,'2026-10-05 00:00:00'),\
     (6,'C3','BOOK','axxb','s3',1,6,103,99,'2026-10-06 00:00:00')",
    "INSERT INTO dw.dim_store VALUES ('s1','central','r1'),('s2','harbor','r2'),('s3','uptown','r1')",
    "INSERT INTO dw.dim_region VALUES ('r1','north'),('r2','south')",
];

fn sales(rest: &str) -> String {
    format!(r#"{{"measures":["sales.amount"],{rest}}}"#)
}

fn grouped(unit: &str, range: &str) -> String {
    sales(&format!(
        r#""timeDimensions":[{{"dimension":"sales.sold_at","granularity":"{unit}","dateRange":{range}}}]"#
    ))
}

fn bucket(name: &'static str, unit: &str, rows: &[(&str, u32)]) -> Case {
    Case {
        name,
        scenario: TEMPORAL,
        fixture: TOKYO_OCTOBER,
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

fn total(name: &'static str, filters: &str, amount: u32) -> Case {
    Case {
        name,
        scenario: TEMPORAL,
        fixture: TOKYO_OCTOBER,
        query: sales(&format!(r#""filters":[{filters}]"#)),
        timezone: "Asia/Tokyo",
        security: "",
        columns: vec!["sales__amount".into()],
        rows: vec![vec![Value::from(amount)]],
    }
}

fn date_filter(name: &'static str, values: &str, amount: u32) -> Case {
    let filter = format!(r#"{{"member":"sales.sold_at","operator":"{name}","values":{values}}}"#);
    total(name, &filter, amount)
}

/// A case on the retail fixture in UTC: `query` JSON, expected columns and rows.
fn retail(name: &'static str, query: &str, columns: &[&str], rows: Value) -> Case {
    let Value::Array(rows) = rows else {
        unreachable!("rows are written as an array")
    };
    Case {
        name,
        scenario: MATRIX,
        fixture: RETAIL,
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
fn passing(name: &'static str, filters: &str, amount: u32) -> Case {
    retail(
        name,
        &sales(&format!(r#""filters":[{filters}]"#)),
        &["sales__amount"],
        json!([[amount]]),
    )
}

fn region(name: &'static str, operator: &str, amount: u32) -> Case {
    // Upper case, so a match also proves case-insensitive comparison.
    let filter =
        format!(r#"{{"member":"sales.region_cd","operator":"{operator}","values":["A%_\\B"]}}"#);
    passing(name, &filter, amount)
}

fn quantity(name: &'static str, operator: &str, amount: u32) -> Case {
    let filter = format!(r#"{{"member":"sales.qty","operator":"{operator}","values":["3"]}}"#);
    passing(name, &filter, amount)
}

fn temporal() -> Vec<Case> {
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
        Case {
            security: r#"{"filters":[{"member":"sales.sold_at","operator":"inDateRange","values":["2026-10-01","2026-10-31"]}]}"#,
            ..total(
                "security",
                r#"{"or":[{"member":"sales.sold_at","operator":"beforeDate","values":["2026-10-01"]},{"member":"sales.sold_at","operator":"afterOrOnDate","values":["2026-10-01"]}]}"#,
                27,
            )
        },
        Case {
            name: "dst",
            fixture: NEW_YORK_DST,
            query: grouped("day", r#"["2026-11-01","2026-11-01"]"#),
            timezone: "America/New_York",
            rows: vec![vec![midnight("2026-11-01"), json!(15)]],
            ..bucket("dst", "day", &[])
        },
    ]
}

fn matrix() -> Vec<Case> {
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
        retail(
            "plain_range",
            &sales(
                r#""timeDimensions":[{"dimension":"sales.sold_at","dateRange":["2026-10-02","2026-10-04"]}]"#,
            ),
            &["sales__amount"],
            json!([[32]]),
        ),
        // The context zone alone would select the sale of 2 October instead.
        Case {
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
        Case {
            security: r#"{"filters":[{"member":"regions.region_name","operator":"equals","values":["north"]}]}"#,
            ..passing(
                "security_two_hop_join",
                r#"{"member":"sales.is_valid","operator":"equals","values":["true"]}"#,
                140,
            )
        },
        // The user OR admits five sales; the security pattern admits only `a%_\b`.
        Case {
            security: r#"{"filters":[{"member":"sales.region_cd","operator":"startsWith","values":["a%"]}]}"#,
            ..passing(
                "security_pattern",
                r#"{"or":[{"member":"sales.region_cd","operator":"contains","values":["north"]},{"member":"sales.channel_cd","operator":"equals","values":["C3"]}]}"#,
                11,
            )
        },
    ]
}

// scenario: B0-DORIS-010
// scenario: B0-DORIS-002
// Executed on a real engine by the Doris container tier, not by T0.
pub fn cases() -> Vec<Case> {
    let mut cases = temporal();
    cases.extend(matrix());
    cases
}
