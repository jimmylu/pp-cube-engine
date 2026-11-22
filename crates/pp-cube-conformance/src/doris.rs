//! Doris execution cases.

use serde_json::{json, Value};

use crate::{midnight, Case};

pub const MODEL: &str = include_str!("../fixtures/sales.yaml");

const TEMPORAL: &str = "B0-DORIS-010";
const DATABASE: &str = "CREATE DATABASE IF NOT EXISTS dw";
const TABLE: &str =
    "CREATE TABLE IF NOT EXISTS dw.sales_dtl (sold_at DATETIMEV2(3), amount DOUBLE) \
     DUPLICATE KEY(sold_at) DISTRIBUTED BY HASH(sold_at) BUCKETS 1 \
     PROPERTIES ('replication_num'='1')";
const EMPTY: &str = "TRUNCATE TABLE dw.sales_dtl";

/// UTC instants one millisecond either side of the Tokyo October boundaries.
const TOKYO_OCTOBER: &[&str] = &[
    DATABASE,
    TABLE,
    EMPTY,
    "INSERT INTO dw.sales_dtl VALUES ('2026-09-30 14:59:59.999',10),\
     ('2026-09-30 15:00:00.000',20),('2026-10-31 14:59:59.999',7),\
     ('2026-10-31 15:00:00.000',100)",
];

/// New York leaves daylight time on 2026-11-01, a 25-hour local day.
const NEW_YORK_DST: &[&str] = &[
    DATABASE,
    TABLE,
    EMPTY,
    "INSERT INTO dw.sales_dtl VALUES ('2026-11-01 04:00:00',3),('2026-11-01 05:30:00',5),\
     ('2026-11-01 06:30:00',7),('2026-11-02 05:00:00',100)",
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

// scenario: B0-DORIS-010
// Executed on a real engine by the Doris container tier, not by T0.
pub fn cases() -> Vec<Case> {
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
