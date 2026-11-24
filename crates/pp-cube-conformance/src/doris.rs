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
     store_name VARCHAR(32), region_cd VARCHAR(16), floor_area DOUBLE) DUPLICATE KEY(store_cd) \
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
const REFUNDS: &str = "CREATE TABLE IF NOT EXISTS dw.refund_dtl (refund_id INT, \
     store_cd VARCHAR(16), amount DOUBLE) DUPLICATE KEY(refund_id) \
     DISTRIBUTED BY HASH(refund_id) BUCKETS 1 PROPERTIES ('replication_num'='1')";
const VISITS: &str = "CREATE TABLE IF NOT EXISTS dw.visit (visit_id INT, store_cd VARCHAR(16)) \
     DUPLICATE KEY(visit_id) DISTRIBUTED BY HASH(visit_id) BUCKETS 1 \
     PROPERTIES ('replication_num'='1')";
const NO_SALES: &str = "TRUNCATE TABLE dw.sales_dtl";
const NO_STORES: &str = "TRUNCATE TABLE dw.dim_store";
const NO_REGIONS: &str = "TRUNCATE TABLE dw.dim_region";
const NO_RECEIPTS: &str = "TRUNCATE TABLE dw.receipt";
const NO_PROMOS: &str = "TRUNCATE TABLE dw.promo";
const NO_REFUNDS: &str = "TRUNCATE TABLE dw.refund_dtl";
const NO_VISITS: &str = "TRUNCATE TABLE dw.visit";

const TOKYO_OCTOBER: &[&str] = &[
    DATABASE,
    SALES,
    STORES,
    REGIONS,
    RECEIPTS,
    PROMOS,
    REFUNDS,
    VISITS,
    NO_SALES,
    NO_STORES,
    NO_REGIONS,
    NO_RECEIPTS,
    NO_PROMOS,
    NO_REFUNDS,
    NO_VISITS,
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
    REFUNDS,
    VISITS,
    NO_SALES,
    NO_STORES,
    NO_REGIONS,
    NO_RECEIPTS,
    NO_PROMOS,
    NO_REFUNDS,
    NO_VISITS,
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
    REFUNDS,
    VISITS,
    NO_SALES,
    NO_STORES,
    NO_REGIONS,
    NO_RECEIPTS,
    NO_PROMOS,
    NO_REFUNDS,
    NO_VISITS,
    "INSERT INTO dw.sales_dtl VALUES \
     (1,'C1','TOY','north','s1',1,1,100,10,'2026-10-01 00:00:00','R1','P1'),\
     (2,'C1','TOY','north','s1',1,2,100,20,'2026-10-02 00:00:00','R2','P1'),\
     (3,'C2','BOOK','south','s2',1,3,101,5,'2026-10-03 00:00:00','R3','P1'),\
     (4,'C2','TOY',NULL,'s2',0,4,102,7,'2026-10-04 00:00:00',NULL,NULL),\
     (5,'C3','BOOK','a%_\\\\b','s3',1,5,103,11,'2026-10-05 00:00:00','R5','P2'),\
     (6,'C3','BOOK','axxb','s3',1,6,103,99,'2026-10-06 00:00:00','R6','P2')",
    "INSERT INTO dw.dim_store VALUES ('s1','central','r1',100),('s2','harbor','r2',200),\
     ('s3','uptown','r1',50),('s4',NULL,'r2',70)",
    "INSERT INTO dw.dim_region VALUES ('r1','north'),('r2','south')",
    "INSERT INTO dw.receipt VALUES ('R1',1),('R2',2),('R3',5),('R5',3),('R6',9),('R9',100)",
    "INSERT INTO dw.promo VALUES ('P1','r1','spring-north'),('P1','r2','spring-south'),\
     ('P2','r2','autumn-south')",
    "INSERT INTO dw.refund_dtl VALUES (1,'s1',4),(2,'s1',6),(3,'s2',5),(4,'s4',8)",
    "INSERT INTO dw.visit VALUES (1,'s1'),(2,'s3'),(3,'s4'),(4,'s4')",
];

/// Nine policy terms of four products, three of them renewals, with their
/// earned premium, six claims, four reserves, eight payments and five cessions.
const INSURANCE: &[&str] = &[
    "CREATE DATABASE IF NOT EXISTS dw",
    "CREATE TABLE IF NOT EXISTS dw.ins_policy (policy_id INT, policy_no VARCHAR(16), product_cd VARCHAR(16), channel_cd VARCHAR(16), branch_cd VARCHAR(16), insured_id INT, insured_type VARCHAR(16), renewal_of INT, status VARCHAR(16), sum_insured DOUBLE, written_premium DOUBLE, issued_at DATETIMEV2(3)) DUPLICATE KEY(policy_id) DISTRIBUTED BY HASH(policy_id) BUCKETS 1 PROPERTIES ('replication_num'='1')",
    "TRUNCATE TABLE dw.ins_policy",
    "INSERT INTO dw.ins_policy VALUES (1,'POL-001','P_MOT','AG','B1',101,'PERSONAL',NULL,'EXPIRED',20000,1200,'2025-02-10 00:00:00'),(2,'POL-002','P_HOME','BR','B2',102,'PERSONAL',NULL,'EXPIRED',300000,900,'2025-05-20 00:00:00'),(3,'POL-003','P_LIAB','BR','B1',103,'COMMERCIAL',NULL,'EXPIRED',1000000,5000,'2025-03-15 00:00:00'),(4,'POL-004','P_MOT','AG','B1',101,'PERSONAL',1,'IN_FORCE',20000,1300,'2026-02-10 00:00:00'),(5,'POL-005','P_HOME','BR','B2',102,'PERSONAL',2,'IN_FORCE',300000,950,'2026-05-20 00:00:00'),(6,'POL-006','P_LIAB','BR','B1',103,'COMMERCIAL',3,'IN_FORCE',1000000,5500,'2026-03-15 00:00:00'),(7,'POL-007','P_TPL','DI','B3',104,'PERSONAL',NULL,'IN_FORCE',50000,800,'2026-01-05 00:00:00'),(8,'POL-008','P_MOT','AG','B3',105,'COMMERCIAL',NULL,'IN_FORCE',40000,2400,'2026-04-01 00:00:00'),(9,'POL-009','P_HOME','DI','B3',106,'PERSONAL',NULL,'CANCELLED',250000,850,'2026-02-25 00:00:00')",
    "CREATE TABLE IF NOT EXISTS dw.ins_earned_premium (earned_id INT, policy_id INT, earned_month DATETIMEV2(3), earned_premium DOUBLE, exposure DOUBLE) DUPLICATE KEY(earned_id) DISTRIBUTED BY HASH(earned_id) BUCKETS 1 PROPERTIES ('replication_num'='1')",
    "TRUNCATE TABLE dw.ins_earned_premium",
    "INSERT INTO dw.ins_earned_premium VALUES (1,1,'2025-12-01 00:00:00',100,1),(2,1,'2026-01-01 00:00:00',100,1),(3,4,'2026-03-01 00:00:00',110,1),(4,4,'2026-04-01 00:00:00',110,1),(5,5,'2026-06-01 00:00:00',80,1),(6,6,'2026-04-01 00:00:00',460,1),(7,6,'2026-05-01 00:00:00',460,1),(8,7,'2026-02-01 00:00:00',70,1),(9,7,'2026-03-01 00:00:00',70,1),(10,8,'2026-05-01 00:00:00',200,2),(11,8,'2026-06-01 00:00:00',200,2),(12,3,'2026-01-01 00:00:00',420,1)",
    "CREATE TABLE IF NOT EXISTS dw.ins_claim (claim_id INT, claim_no VARCHAR(16), policy_id INT, cause_cd VARCHAR(16), status VARCHAR(16), loss_at DATETIMEV2(3), reported_at DATETIMEV2(3), report_lag_days INT, is_large_loss INT) DUPLICATE KEY(claim_id) DISTRIBUTED BY HASH(claim_id) BUCKETS 1 PROPERTIES ('replication_num'='1')",
    "TRUNCATE TABLE dw.ins_claim",
    "INSERT INTO dw.ins_claim VALUES (1,'CLM-001',1,'COLLISION','CLOSED','2025-06-10 00:00:00','2025-06-12 00:00:00',2,0),(2,'CLM-002',3,'INJURY','OPEN','2025-11-20 00:00:00','2025-12-05 00:00:00',15,1),(3,'CLM-003',4,'COLLISION','CLOSED','2026-03-03 00:00:00','2026-03-04 00:00:00',1,0),(4,'CLM-004',4,'THEFT','OPEN','2026-04-18 00:00:00','2026-04-21 00:00:00',3,1),(5,'CLM-005',5,'WATER','OPEN','2026-06-02 00:00:00','2026-06-09 00:00:00',7,0),(6,'CLM-006',8,'COLLISION','OPEN','2026-05-15 00:00:00','2026-05-15 00:00:00',0,0)",
    "CREATE TABLE IF NOT EXISTS dw.ins_claim_reserve (claim_id INT, outstanding DOUBLE) DUPLICATE KEY(claim_id) DISTRIBUTED BY HASH(claim_id) BUCKETS 1 PROPERTIES ('replication_num'='1')",
    "TRUNCATE TABLE dw.ins_claim_reserve",
    "INSERT INTO dw.ins_claim_reserve VALUES (2,8000),(4,15000),(5,600),(6,900)",
    "CREATE TABLE IF NOT EXISTS dw.ins_claim_payment (payment_id INT, claim_id INT, payment_type VARCHAR(16), paid_at DATETIMEV2(3), paid_amount DOUBLE) DUPLICATE KEY(payment_id) DISTRIBUTED BY HASH(payment_id) BUCKETS 1 PROPERTIES ('replication_num'='1')",
    "TRUNCATE TABLE dw.ins_claim_payment",
    "INSERT INTO dw.ins_claim_payment VALUES (1,1,'INDEMNITY','2025-07-01 00:00:00',700),(2,1,'EXPENSE','2025-07-15 00:00:00',50),(3,2,'INDEMNITY','2026-01-20 00:00:00',3000),(4,2,'EXPENSE','2026-02-10 00:00:00',400),(5,3,'INDEMNITY','2026-03-20 00:00:00',500),(6,4,'INDEMNITY','2026-05-05 00:00:00',4000),(7,4,'EXPENSE','2026-06-10 00:00:00',250),(8,5,'INDEMNITY','2026-06-20 00:00:00',300)",
    "CREATE TABLE IF NOT EXISTS dw.ins_cession (cession_id INT, policy_id INT, treaty_cd VARCHAR(16), ceded_premium DOUBLE) DUPLICATE KEY(cession_id) DISTRIBUTED BY HASH(cession_id) BUCKETS 1 PROPERTIES ('replication_num'='1')",
    "TRUNCATE TABLE dw.ins_cession",
    "INSERT INTO dw.ins_cession VALUES (1,3,'QS',1500),(2,6,'QS',1650),(3,6,'XL',300),(4,8,'QS',720),(5,4,'QS',390)",
    "CREATE TABLE IF NOT EXISTS dw.ins_product (product_cd VARCHAR(16), product_name VARCHAR(32), line_of_business VARCHAR(16)) DUPLICATE KEY(product_cd) DISTRIBUTED BY HASH(product_cd) BUCKETS 1 PROPERTIES ('replication_num'='1')",
    "TRUNCATE TABLE dw.ins_product",
    "INSERT INTO dw.ins_product VALUES ('P_MOT','Motor Own Damage','MOTOR'),('P_TPL','Motor Liability','MOTOR'),('P_HOME','Home','PROPERTY'),('P_LIAB','Public Liability','LIABILITY')",
    "CREATE TABLE IF NOT EXISTS dw.ins_channel (channel_cd VARCHAR(16), channel_name VARCHAR(32)) DUPLICATE KEY(channel_cd) DISTRIBUTED BY HASH(channel_cd) BUCKETS 1 PROPERTIES ('replication_num'='1')",
    "TRUNCATE TABLE dw.ins_channel",
    "INSERT INTO dw.ins_channel VALUES ('AG','Agency'),('BR','Broker'),('DI','Direct')",
    "CREATE TABLE IF NOT EXISTS dw.ins_branch (branch_cd VARCHAR(16), branch_name VARCHAR(32), region_name VARCHAR(16)) DUPLICATE KEY(branch_cd) DISTRIBUTED BY HASH(branch_cd) BUCKETS 1 PROPERTIES ('replication_num'='1')",
    "TRUNCATE TABLE dw.ins_branch",
    "INSERT INTO dw.ins_branch VALUES ('B1','East Branch','East'),('B2','North Branch','North'),('B3','Lake Branch','North')",
];

pub fn fixture(fixture: Fixture) -> &'static [&'static str] {
    match fixture {
        Fixture::TokyoOctober => TOKYO_OCTOBER,
        Fixture::NewYorkDst => NEW_YORK_DST,
        Fixture::Retail => RETAIL,
        Fixture::Insurance => INSURANCE,
    }
}

// scenario: B0-DORIS-010
// scenario: B0-DORIS-002
// scenario: B0-DORIS-011
// Executed on a real engine by the Doris execution tier, not by T0.
pub fn scenario(group: Group) -> &'static str {
    match group {
        Group::Temporal => "B0-DORIS-010",
        Group::Matrix | Group::Domain => "B0-DORIS-002",
        Group::EmptyPage => "B0-DORIS-011",
    }
}
