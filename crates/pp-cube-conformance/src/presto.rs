//! Presto fixtures and scenario for the shared execution cases.
//!
//! Fixtures use the memory connector with session catalog `memory`, schema `dw`
//! and time zone UTC. The connector cannot empty a table, so every fixture drops
//! and recreates the tables it loads.

use crate::cases::{Fixture, Group};

const SCHEMA: &str = "CREATE SCHEMA IF NOT EXISTS memory.dw";
const NO_SALES: &str = "DROP TABLE IF EXISTS memory.dw.sales_dtl";
const NO_STORES: &str = "DROP TABLE IF EXISTS memory.dw.dim_store";
const NO_REGIONS: &str = "DROP TABLE IF EXISTS memory.dw.dim_region";
const SALES: &str = "CREATE TABLE memory.dw.sales_dtl (sale_id INTEGER, channel_cd VARCHAR, \
     category_cd VARCHAR, region_cd VARCHAR, store_cd VARCHAR, is_valid INTEGER, qty INTEGER, \
     order_no INTEGER, amount DOUBLE, sold_at TIMESTAMP)";
const STORES: &str =
    "CREATE TABLE memory.dw.dim_store (store_cd VARCHAR, store_name VARCHAR, region_cd VARCHAR)";
const REGIONS: &str = "CREATE TABLE memory.dw.dim_region (region_cd VARCHAR, region_name VARCHAR)";

const TOKYO_OCTOBER: &[&str] = &[
    SCHEMA,
    NO_SALES,
    NO_STORES,
    NO_REGIONS,
    SALES,
    STORES,
    REGIONS,
    "INSERT INTO memory.dw.sales_dtl (sale_id, sold_at, amount) VALUES \
     (1,TIMESTAMP '2026-09-30 14:59:59.999',10),(2,TIMESTAMP '2026-09-30 15:00:00.000',20),\
     (3,TIMESTAMP '2026-10-31 14:59:59.999',7),(4,TIMESTAMP '2026-10-31 15:00:00.000',100)",
];

const NEW_YORK_DST: &[&str] = &[
    SCHEMA,
    NO_SALES,
    NO_STORES,
    NO_REGIONS,
    SALES,
    STORES,
    REGIONS,
    "INSERT INTO memory.dw.sales_dtl (sale_id, sold_at, amount) VALUES \
     (1,TIMESTAMP '2026-11-01 04:00:00',3),(2,TIMESTAMP '2026-11-01 05:30:00',5),\
     (3,TIMESTAMP '2026-11-01 06:30:00',7),(4,TIMESTAMP '2026-11-02 05:00:00',100)",
];

const RETAIL: &[&str] = &[
    SCHEMA,
    NO_SALES,
    NO_STORES,
    NO_REGIONS,
    SALES,
    STORES,
    REGIONS,
    "INSERT INTO memory.dw.sales_dtl VALUES \
     (1,'C1','TOY','north','s1',1,1,100,10,TIMESTAMP '2026-10-01 00:00:00'),\
     (2,'C1','TOY','north','s1',1,2,100,20,TIMESTAMP '2026-10-02 00:00:00'),\
     (3,'C2','BOOK','south','s2',1,3,101,5,TIMESTAMP '2026-10-03 00:00:00'),\
     (4,'C2','TOY',NULL,'s2',0,4,102,7,TIMESTAMP '2026-10-04 00:00:00'),\
     (5,'C3','BOOK','a%_\\b','s3',1,5,103,11,TIMESTAMP '2026-10-05 00:00:00'),\
     (6,'C3','BOOK','axxb','s3',1,6,103,99,TIMESTAMP '2026-10-06 00:00:00')",
    "INSERT INTO memory.dw.dim_store VALUES ('s1','central','r1'),('s2','harbor','r2'),('s3','uptown','r1')",
    "INSERT INTO memory.dw.dim_region VALUES ('r1','north'),('r2','south')",
];

pub fn fixture(fixture: Fixture) -> &'static [&'static str] {
    match fixture {
        Fixture::TokyoOctober => TOKYO_OCTOBER,
        Fixture::NewYorkDst => NEW_YORK_DST,
        Fixture::Retail => RETAIL,
    }
}

// scenario: B0-PRESTO-003
// Executed on a real engine by the Presto execution tier, not by T0.
pub fn scenario(_group: Group) -> &'static str {
    "B0-PRESTO-003"
}
