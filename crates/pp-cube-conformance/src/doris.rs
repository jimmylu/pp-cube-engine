//! Doris fixtures and scenarios for the shared execution cases.
//!
//! Every fixture creates the same tables when absent, empties them and loads
//! its own rows, so a tier can switch fixtures on one engine.

use crate::cases::{Fixture, Group};

const DATABASE: &str = "CREATE DATABASE IF NOT EXISTS dw";
const SALES: &str =
    "CREATE TABLE IF NOT EXISTS dw.sales_dtl (sale_id INT, channel_cd VARCHAR(16), \
     category_cd VARCHAR(16), region_cd VARCHAR(32), store_cd VARCHAR(16), is_valid TINYINT, \
     qty INT, order_no INT, amount DOUBLE, sold_at DATETIMEV2(3), receipt_no VARCHAR(16), \
     promo_cd VARCHAR(16)) DUPLICATE KEY(sale_id) \
     DISTRIBUTED BY HASH(sale_id) BUCKETS 1 PROPERTIES ('replication_num'='1')";
const STORES: &str = "CREATE TABLE IF NOT EXISTS dw.dim_store (store_cd VARCHAR(16), \
     store_name VARCHAR(32), region_cd VARCHAR(16)) DUPLICATE KEY(store_cd) \
     DISTRIBUTED BY HASH(store_cd) BUCKETS 1 PROPERTIES ('replication_num'='1')";
const REGIONS: &str = "CREATE TABLE IF NOT EXISTS dw.dim_region (region_cd VARCHAR(16), \
     region_name VARCHAR(32)) DUPLICATE KEY(region_cd) \
     DISTRIBUTED BY HASH(region_cd) BUCKETS 1 PROPERTIES ('replication_num'='1')";
const RECEIPTS: &str =
    "CREATE TABLE IF NOT EXISTS dw.receipt (receipt_no VARCHAR(16), tax DOUBLE) \
     DUPLICATE KEY(receipt_no) DISTRIBUTED BY HASH(receipt_no) BUCKETS 1 \
     PROPERTIES ('replication_num'='1')";
const PROMOS: &str = "CREATE TABLE IF NOT EXISTS dw.promo (promo_cd VARCHAR(16), \
     region_cd VARCHAR(16), promo_name VARCHAR(32)) DUPLICATE KEY(promo_cd) \
     DISTRIBUTED BY HASH(promo_cd) BUCKETS 1 PROPERTIES ('replication_num'='1')";
const NO_SALES: &str = "TRUNCATE TABLE dw.sales_dtl";
const NO_STORES: &str = "TRUNCATE TABLE dw.dim_store";
const NO_REGIONS: &str = "TRUNCATE TABLE dw.dim_region";
const NO_RECEIPTS: &str = "TRUNCATE TABLE dw.receipt";
const NO_PROMOS: &str = "TRUNCATE TABLE dw.promo";

const TOKYO_OCTOBER: &[&str] = &[
    DATABASE,
    SALES,
    STORES,
    REGIONS,
    RECEIPTS,
    PROMOS,
    NO_SALES,
    NO_STORES,
    NO_REGIONS,
    NO_RECEIPTS,
    NO_PROMOS,
    "INSERT INTO dw.sales_dtl (sale_id, sold_at, amount) VALUES \
     (1,'2026-09-30 14:59:59.999',10),(2,'2026-09-30 15:00:00.000',20),\
     (3,'2026-10-31 14:59:59.999',7),(4,'2026-10-31 15:00:00.000',100)",
];

const NEW_YORK_DST: &[&str] = &[
    DATABASE,
    SALES,
    STORES,
    REGIONS,
    RECEIPTS,
    PROMOS,
    NO_SALES,
    NO_STORES,
    NO_REGIONS,
    NO_RECEIPTS,
    NO_PROMOS,
    "INSERT INTO dw.sales_dtl (sale_id, sold_at, amount) VALUES \
     (1,'2026-11-01 04:00:00',3),(2,'2026-11-01 05:30:00',5),\
     (3,'2026-11-01 06:30:00',7),(4,'2026-11-02 05:00:00',100)",
];

const RETAIL: &[&str] = &[
    DATABASE,
    SALES,
    STORES,
    REGIONS,
    RECEIPTS,
    PROMOS,
    NO_SALES,
    NO_STORES,
    NO_REGIONS,
    NO_RECEIPTS,
    NO_PROMOS,
"INSERT INTO dw.sales_dtl VALUES \
     (1,'C1','TOY','north','s1',1,1,100,10,'2026-10-01 00:00:00','R1','P1'),\
     (2,'C1','TOY','north','s1',1,2,100,20,'2026-10-02 00:00:00','R2','P1'),\
     (3,'C2','BOOK','south','s2',1,3,101,5,'2026-10-03 00:00:00','R3','P1'),\
     (4,'C2','TOY',NULL,'s2',0,4,102,7,'2026-10-04 00:00:00',NULL,NULL),\
     (5,'C3','BOOK','a%_\\\\b','s3',1,5,103,11,'2026-10-05 00:00:00','R5','P2'),\
     (6,'C3','BOOK','axxb','s3',1,6,103,99,'2026-10-06 00:00:00','R6','P2')",
    "INSERT INTO dw.dim_store VALUES ('s1','central','r1'),('s2','harbor','r2'),('s3','uptown','r1')",
    "INSERT INTO dw.dim_region VALUES ('r1','north'),('r2','south')",
    "INSERT INTO dw.receipt VALUES ('R1',1),('R2',2),('R3',5),('R5',3),('R6',9),('R9',100)",
    "INSERT INTO dw.promo VALUES ('P1','r1','spring-north'),('P1','r2','spring-south'),\
     ('P2','r2','autumn-south')",
];

pub fn fixture(fixture: Fixture) -> &'static [&'static str] {
    match fixture {
        Fixture::TokyoOctober => TOKYO_OCTOBER,
        Fixture::NewYorkDst => NEW_YORK_DST,
        Fixture::Retail => RETAIL,
    }
}

// scenario: B0-DORIS-010
// scenario: B0-DORIS-002
// scenario: B0-DORIS-011
// Executed on a real engine by the Doris execution tier, not by T0.
pub fn scenario(group: Group) -> &'static str {
    match group {
        Group::Temporal => "B0-DORIS-010",
        Group::Matrix => "B0-DORIS-002",
        Group::EmptyPage => "B0-DORIS-011",
    }
}
