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
const NO_RECEIPTS: &str = "DROP TABLE IF EXISTS memory.dw.receipt";
const NO_PROMOS: &str = "DROP TABLE IF EXISTS memory.dw.promo";
const NO_REFUNDS: &str = "DROP TABLE IF EXISTS memory.dw.refund_dtl";
const NO_VISITS: &str = "DROP TABLE IF EXISTS memory.dw.visit";
const REFUNDS: &str =
    "CREATE TABLE memory.dw.refund_dtl (refund_id INTEGER, store_cd VARCHAR, amount DOUBLE)";
const VISITS: &str = "CREATE TABLE memory.dw.visit (visit_id INTEGER, store_cd VARCHAR)";
const RECEIPTS: &str = "CREATE TABLE memory.dw.receipt (receipt_no VARCHAR, tax DOUBLE)";
const PROMOS: &str =
    "CREATE TABLE memory.dw.promo (promo_cd VARCHAR, region_cd VARCHAR, promo_name VARCHAR)";
const SALES: &str = "CREATE TABLE memory.dw.sales_dtl (sale_id INTEGER, channel_cd VARCHAR, \
     category_cd VARCHAR, region_cd VARCHAR, store_cd VARCHAR, is_valid INTEGER, qty INTEGER, \
     order_no INTEGER, amount DOUBLE, sold_at TIMESTAMP, receipt_no VARCHAR, promo_cd VARCHAR)";
const STORES: &str = "CREATE TABLE memory.dw.dim_store (store_cd VARCHAR, store_name VARCHAR, \
     region_cd VARCHAR, floor_area DOUBLE)";
const REGIONS: &str = "CREATE TABLE memory.dw.dim_region (region_cd VARCHAR, region_name VARCHAR)";

const TOKYO_OCTOBER: &[&str] = &[
    SCHEMA,
    NO_SALES,
    NO_STORES,
    NO_REGIONS,
    NO_RECEIPTS,
    NO_PROMOS,
    NO_REFUNDS,
    NO_VISITS,
    SALES,
    STORES,
    REGIONS,
    RECEIPTS,
    PROMOS,
    REFUNDS,
    VISITS,
    "INSERT INTO memory.dw.sales_dtl (sale_id, sold_at, amount) VALUES \
     (1,TIMESTAMP '2026-09-30 14:59:59.999',10),(2,TIMESTAMP '2026-09-30 15:00:00.000',20),\
     (3,TIMESTAMP '2026-10-31 14:59:59.999',7),(4,TIMESTAMP '2026-10-31 15:00:00.000',100)",
];

const NEW_YORK_DST: &[&str] = &[
    SCHEMA,
    NO_SALES,
    NO_STORES,
    NO_REGIONS,
    NO_RECEIPTS,
    NO_PROMOS,
    NO_REFUNDS,
    NO_VISITS,
    SALES,
    STORES,
    REGIONS,
    RECEIPTS,
    PROMOS,
    REFUNDS,
    VISITS,
    "INSERT INTO memory.dw.sales_dtl (sale_id, sold_at, amount) VALUES \
     (1,TIMESTAMP '2026-11-01 04:00:00',3),(2,TIMESTAMP '2026-11-01 05:30:00',5),\
     (3,TIMESTAMP '2026-11-01 06:30:00',7),(4,TIMESTAMP '2026-11-02 05:00:00',100)",
];

const RETAIL: &[&str] = &[
    SCHEMA,
    NO_SALES,
    NO_STORES,
    NO_REGIONS,
    NO_RECEIPTS,
    NO_PROMOS,
    NO_REFUNDS,
    NO_VISITS,
    SALES,
    STORES,
    REGIONS,
    RECEIPTS,
    PROMOS,
    REFUNDS,
    VISITS,
    "INSERT INTO memory.dw.sales_dtl VALUES \
     (1,'C1','TOY','north','s1',1,1,100,10,TIMESTAMP '2026-10-01 00:00:00','R1','P1'),\
     (2,'C1','TOY','north','s1',1,2,100,20,TIMESTAMP '2026-10-02 00:00:00','R2','P1'),\
     (3,'C2','BOOK','south','s2',1,3,101,5,TIMESTAMP '2026-10-03 00:00:00','R3','P1'),\
     (4,'C2','TOY',NULL,'s2',0,4,102,7,TIMESTAMP '2026-10-04 00:00:00',NULL,NULL),\
     (5,'C3','BOOK','a%_\\b','s3',1,5,103,11,TIMESTAMP '2026-10-05 00:00:00','R5','P2'),\
     (6,'C3','BOOK','axxb','s3',1,6,103,99,TIMESTAMP '2026-10-06 00:00:00','R6','P2')",
    "INSERT INTO memory.dw.dim_store VALUES ('s1','central','r1',100),('s2','harbor','r2',200),\
     ('s3','uptown','r1',50),('s4',NULL,'r2',70)",
    "INSERT INTO memory.dw.dim_region VALUES ('r1','north'),('r2','south')",
    "INSERT INTO memory.dw.receipt VALUES ('R1',1),('R2',2),('R3',5),('R5',3),('R6',9),('R9',100)",
    "INSERT INTO memory.dw.promo VALUES ('P1','r1','spring-north'),('P1','r2','spring-south'),\
     ('P2','r2','autumn-south')",
    "INSERT INTO memory.dw.refund_dtl VALUES (1,'s1',4),(2,'s1',6),(3,'s2',5),(4,'s4',8)",
    "INSERT INTO memory.dw.visit VALUES (1,'s1'),(2,'s3'),(3,'s4'),(4,'s4')",
];

/// Nine policy terms of four products, three of them renewals, with their
/// earned premium, six claims, four reserves, eight payments and five cessions.
const INSURANCE: &[&str] = &[
    "CREATE SCHEMA IF NOT EXISTS memory.dw",
    "DROP TABLE IF EXISTS memory.dw.ins_policy",
    "CREATE TABLE memory.dw.ins_policy (policy_id INTEGER, policy_no VARCHAR, product_cd VARCHAR, channel_cd VARCHAR, branch_cd VARCHAR, insured_id INTEGER, insured_type VARCHAR, renewal_of INTEGER, status VARCHAR, sum_insured DOUBLE, written_premium DOUBLE, issued_at TIMESTAMP)",
    "INSERT INTO memory.dw.ins_policy VALUES (1,'POL-001','P_MOT','AG','B1',101,'PERSONAL',NULL,'EXPIRED',20000,1200,TIMESTAMP '2025-02-10 00:00:00'),(2,'POL-002','P_HOME','BR','B2',102,'PERSONAL',NULL,'EXPIRED',300000,900,TIMESTAMP '2025-05-20 00:00:00'),(3,'POL-003','P_LIAB','BR','B1',103,'COMMERCIAL',NULL,'EXPIRED',1000000,5000,TIMESTAMP '2025-03-15 00:00:00'),(4,'POL-004','P_MOT','AG','B1',101,'PERSONAL',1,'IN_FORCE',20000,1300,TIMESTAMP '2026-02-10 00:00:00'),(5,'POL-005','P_HOME','BR','B2',102,'PERSONAL',2,'IN_FORCE',300000,950,TIMESTAMP '2026-05-20 00:00:00'),(6,'POL-006','P_LIAB','BR','B1',103,'COMMERCIAL',3,'IN_FORCE',1000000,5500,TIMESTAMP '2026-03-15 00:00:00'),(7,'POL-007','P_TPL','DI','B3',104,'PERSONAL',NULL,'IN_FORCE',50000,800,TIMESTAMP '2026-01-05 00:00:00'),(8,'POL-008','P_MOT','AG','B3',105,'COMMERCIAL',NULL,'IN_FORCE',40000,2400,TIMESTAMP '2026-04-01 00:00:00'),(9,'POL-009','P_HOME','DI','B3',106,'PERSONAL',NULL,'CANCELLED',250000,850,TIMESTAMP '2026-02-25 00:00:00')",
    "DROP TABLE IF EXISTS memory.dw.ins_earned_premium",
    "CREATE TABLE memory.dw.ins_earned_premium (earned_id INTEGER, policy_id INTEGER, earned_month TIMESTAMP, earned_premium DOUBLE, exposure DOUBLE)",
    "INSERT INTO memory.dw.ins_earned_premium VALUES (1,1,TIMESTAMP '2025-12-01 00:00:00',100,1),(2,1,TIMESTAMP '2026-01-01 00:00:00',100,1),(3,4,TIMESTAMP '2026-03-01 00:00:00',110,1),(4,4,TIMESTAMP '2026-04-01 00:00:00',110,1),(5,5,TIMESTAMP '2026-06-01 00:00:00',80,1),(6,6,TIMESTAMP '2026-04-01 00:00:00',460,1),(7,6,TIMESTAMP '2026-05-01 00:00:00',460,1),(8,7,TIMESTAMP '2026-02-01 00:00:00',70,1),(9,7,TIMESTAMP '2026-03-01 00:00:00',70,1),(10,8,TIMESTAMP '2026-05-01 00:00:00',200,2),(11,8,TIMESTAMP '2026-06-01 00:00:00',200,2),(12,3,TIMESTAMP '2026-01-01 00:00:00',420,1)",
    "DROP TABLE IF EXISTS memory.dw.ins_claim",
    "CREATE TABLE memory.dw.ins_claim (claim_id INTEGER, claim_no VARCHAR, policy_id INTEGER, cause_cd VARCHAR, status VARCHAR, loss_at TIMESTAMP, reported_at TIMESTAMP, report_lag_days INTEGER, is_large_loss INTEGER)",
    "INSERT INTO memory.dw.ins_claim VALUES (1,'CLM-001',1,'COLLISION','CLOSED',TIMESTAMP '2025-06-10 00:00:00',TIMESTAMP '2025-06-12 00:00:00',2,0),(2,'CLM-002',3,'INJURY','OPEN',TIMESTAMP '2025-11-20 00:00:00',TIMESTAMP '2025-12-05 00:00:00',15,1),(3,'CLM-003',4,'COLLISION','CLOSED',TIMESTAMP '2026-03-03 00:00:00',TIMESTAMP '2026-03-04 00:00:00',1,0),(4,'CLM-004',4,'THEFT','OPEN',TIMESTAMP '2026-04-18 00:00:00',TIMESTAMP '2026-04-21 00:00:00',3,1),(5,'CLM-005',5,'WATER','OPEN',TIMESTAMP '2026-06-02 00:00:00',TIMESTAMP '2026-06-09 00:00:00',7,0),(6,'CLM-006',8,'COLLISION','OPEN',TIMESTAMP '2026-05-15 00:00:00',TIMESTAMP '2026-05-15 00:00:00',0,0)",
    "DROP TABLE IF EXISTS memory.dw.ins_claim_reserve",
    "CREATE TABLE memory.dw.ins_claim_reserve (claim_id INTEGER, outstanding DOUBLE)",
    "INSERT INTO memory.dw.ins_claim_reserve VALUES (2,8000),(4,15000),(5,600),(6,900)",
    "DROP TABLE IF EXISTS memory.dw.ins_claim_payment",
    "CREATE TABLE memory.dw.ins_claim_payment (payment_id INTEGER, claim_id INTEGER, payment_type VARCHAR, paid_at TIMESTAMP, paid_amount DOUBLE)",
    "INSERT INTO memory.dw.ins_claim_payment VALUES (1,1,'INDEMNITY',TIMESTAMP '2025-07-01 00:00:00',700),(2,1,'EXPENSE',TIMESTAMP '2025-07-15 00:00:00',50),(3,2,'INDEMNITY',TIMESTAMP '2026-01-20 00:00:00',3000),(4,2,'EXPENSE',TIMESTAMP '2026-02-10 00:00:00',400),(5,3,'INDEMNITY',TIMESTAMP '2026-03-20 00:00:00',500),(6,4,'INDEMNITY',TIMESTAMP '2026-05-05 00:00:00',4000),(7,4,'EXPENSE',TIMESTAMP '2026-06-10 00:00:00',250),(8,5,'INDEMNITY',TIMESTAMP '2026-06-20 00:00:00',300)",
    "DROP TABLE IF EXISTS memory.dw.ins_cession",
    "CREATE TABLE memory.dw.ins_cession (cession_id INTEGER, policy_id INTEGER, treaty_cd VARCHAR, ceded_premium DOUBLE)",
    "INSERT INTO memory.dw.ins_cession VALUES (1,3,'QS',1500),(2,6,'QS',1650),(3,6,'XL',300),(4,8,'QS',720),(5,4,'QS',390)",
    "DROP TABLE IF EXISTS memory.dw.ins_product",
    "CREATE TABLE memory.dw.ins_product (product_cd VARCHAR, product_name VARCHAR, line_of_business VARCHAR)",
    "INSERT INTO memory.dw.ins_product VALUES ('P_MOT','Motor Own Damage','MOTOR'),('P_TPL','Motor Liability','MOTOR'),('P_HOME','Home','PROPERTY'),('P_LIAB','Public Liability','LIABILITY')",
    "DROP TABLE IF EXISTS memory.dw.ins_channel",
    "CREATE TABLE memory.dw.ins_channel (channel_cd VARCHAR, channel_name VARCHAR)",
    "INSERT INTO memory.dw.ins_channel VALUES ('AG','Agency'),('BR','Broker'),('DI','Direct')",
    "DROP TABLE IF EXISTS memory.dw.ins_branch",
    "CREATE TABLE memory.dw.ins_branch (branch_cd VARCHAR, branch_name VARCHAR, region_name VARCHAR)",
    "INSERT INTO memory.dw.ins_branch VALUES ('B1','East Branch','East'),('B2','North Branch','North'),('B3','Lake Branch','North')",
];

pub fn fixture(fixture: Fixture) -> &'static [&'static str] {
    match fixture {
        Fixture::TokyoOctober => TOKYO_OCTOBER,
        Fixture::NewYorkDst => NEW_YORK_DST,
        Fixture::Retail => RETAIL,
        Fixture::Insurance => INSURANCE,
    }
}

// scenario: B0-PRESTO-003
// scenario: B0-PRESTO-002
// Executed on a real engine by the Presto execution tier, not by T0.
pub fn scenario(_group: Group) -> &'static str {
    "B0-PRESTO-003"
}
