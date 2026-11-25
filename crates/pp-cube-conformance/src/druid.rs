//! Druid fixtures and scenario for the shared execution cases.
//!
//! Druid has no schema but its own and no table without rows. A fixture is one
//! statement a table that has rows: a SQL ingestion that replaces the
//! datasource with rows written inline. An execution tier submits each as an
//! ingestion task and waits until the new segments, and no older ones, are
//! served. The statements are generated from the Presto fixtures, so the three
//! engines hold the same rows.
//!
//! Each table's time column is ingested as `__time`, the one timestamp column a
//! datasource has; `model` rewrites the shared model to match. Tables without
//! a time column get a constant.

use crate::cases::{Fixture, Group};

/// The shared model as a Druid deployment would write it: datasources live in
/// the schema `druid`, and a cube's time dimension is `__time`.
pub fn model(shared: &str) -> String {
    let mut text = shared.replace("dw.", "druid.");
    for column in ["sold_at", "issued_at", "earned_month", "loss_at", "paid_at"] {
        text = text.replace(&format!("{{CUBE}}.{column}\""), "{CUBE}.__time\"");
    }
    text
}

/// Four sales around the Tokyo October boundaries.
const TOKYO_OCTOBER: &[&str] = &[
    r####"REPLACE INTO "sales_dtl" OVERWRITE ALL SELECT "sale_id", "amount", TIME_PARSE("sold_at") AS "__time" FROM TABLE(EXTERN('{"type":"inline","data":"{\"sale_id\":1,\"amount\":10,\"sold_at\":\"2026-09-30T14:59:59.999\"}\n{\"sale_id\":2,\"amount\":20,\"sold_at\":\"2026-09-30T15:00:00.000\"}\n{\"sale_id\":3,\"amount\":7,\"sold_at\":\"2026-10-31T14:59:59.999\"}\n{\"sale_id\":4,\"amount\":100,\"sold_at\":\"2026-10-31T15:00:00.000\"}"}', '{"type":"json"}', '[{"name":"sale_id","type":"long"},{"name":"amount","type":"double"},{"name":"sold_at","type":"string"}]')) PARTITIONED BY ALL"####,
];

/// Four sales around the New York daylight-saving day.
const NEW_YORK_DST: &[&str] = &[
    r####"REPLACE INTO "sales_dtl" OVERWRITE ALL SELECT "sale_id", "amount", TIME_PARSE("sold_at") AS "__time" FROM TABLE(EXTERN('{"type":"inline","data":"{\"sale_id\":1,\"amount\":3,\"sold_at\":\"2026-11-01T04:00:00\"}\n{\"sale_id\":2,\"amount\":5,\"sold_at\":\"2026-11-01T05:30:00\"}\n{\"sale_id\":3,\"amount\":7,\"sold_at\":\"2026-11-01T06:30:00\"}\n{\"sale_id\":4,\"amount\":100,\"sold_at\":\"2026-11-02T05:00:00\"}"}', '{"type":"json"}', '[{"name":"sale_id","type":"long"},{"name":"amount","type":"double"},{"name":"sold_at","type":"string"}]')) PARTITIONED BY ALL"####,
];

/// The retail tables that have rows.
const RETAIL: &[&str] = &[
    r####"REPLACE INTO "sales_dtl" OVERWRITE ALL SELECT "sale_id", "channel_cd", "category_cd", "region_cd", "store_cd", "is_valid", "qty", "order_no", "amount", TIME_PARSE("sold_at") AS "__time", "receipt_no", "promo_cd" FROM TABLE(EXTERN('{"type":"inline","data":"{\"sale_id\":1,\"channel_cd\":\"C1\",\"category_cd\":\"TOY\",\"region_cd\":\"north\",\"store_cd\":\"s1\",\"is_valid\":1,\"qty\":1,\"order_no\":100,\"amount\":10,\"sold_at\":\"2026-10-01T00:00:00\",\"receipt_no\":\"R1\",\"promo_cd\":\"P1\"}\n{\"sale_id\":2,\"channel_cd\":\"C1\",\"category_cd\":\"TOY\",\"region_cd\":\"north\",\"store_cd\":\"s1\",\"is_valid\":1,\"qty\":2,\"order_no\":100,\"amount\":20,\"sold_at\":\"2026-10-02T00:00:00\",\"receipt_no\":\"R2\",\"promo_cd\":\"P1\"}\n{\"sale_id\":3,\"channel_cd\":\"C2\",\"category_cd\":\"BOOK\",\"region_cd\":\"south\",\"store_cd\":\"s2\",\"is_valid\":1,\"qty\":3,\"order_no\":101,\"amount\":5,\"sold_at\":\"2026-10-03T00:00:00\",\"receipt_no\":\"R3\",\"promo_cd\":\"P1\"}\n{\"sale_id\":4,\"channel_cd\":\"C2\",\"category_cd\":\"TOY\",\"region_cd\":null,\"store_cd\":\"s2\",\"is_valid\":0,\"qty\":4,\"order_no\":102,\"amount\":7,\"sold_at\":\"2026-10-04T00:00:00\",\"receipt_no\":null,\"promo_cd\":null}\n{\"sale_id\":5,\"channel_cd\":\"C3\",\"category_cd\":\"BOOK\",\"region_cd\":\"a%_\\\\b\",\"store_cd\":\"s3\",\"is_valid\":1,\"qty\":5,\"order_no\":103,\"amount\":11,\"sold_at\":\"2026-10-05T00:00:00\",\"receipt_no\":\"R5\",\"promo_cd\":\"P2\"}\n{\"sale_id\":6,\"channel_cd\":\"C3\",\"category_cd\":\"BOOK\",\"region_cd\":\"axxb\",\"store_cd\":\"s3\",\"is_valid\":1,\"qty\":6,\"order_no\":103,\"amount\":99,\"sold_at\":\"2026-10-06T00:00:00\",\"receipt_no\":\"R6\",\"promo_cd\":\"P2\"}"}', '{"type":"json"}', '[{"name":"sale_id","type":"long"},{"name":"channel_cd","type":"string"},{"name":"category_cd","type":"string"},{"name":"region_cd","type":"string"},{"name":"store_cd","type":"string"},{"name":"is_valid","type":"long"},{"name":"qty","type":"long"},{"name":"order_no","type":"long"},{"name":"amount","type":"double"},{"name":"sold_at","type":"string"},{"name":"receipt_no","type":"string"},{"name":"promo_cd","type":"string"}]')) PARTITIONED BY ALL"####,
    r####"REPLACE INTO "dim_store" OVERWRITE ALL SELECT TIMESTAMP '2000-01-01 00:00:00' AS "__time", "store_cd", "store_name", "region_cd", "floor_area" FROM TABLE(EXTERN('{"type":"inline","data":"{\"store_cd\":\"s1\",\"store_name\":\"central\",\"region_cd\":\"r1\",\"floor_area\":100}\n{\"store_cd\":\"s2\",\"store_name\":\"harbor\",\"region_cd\":\"r2\",\"floor_area\":200}\n{\"store_cd\":\"s3\",\"store_name\":\"uptown\",\"region_cd\":\"r1\",\"floor_area\":50}\n{\"store_cd\":\"s4\",\"store_name\":null,\"region_cd\":\"r2\",\"floor_area\":70}"}', '{"type":"json"}', '[{"name":"store_cd","type":"string"},{"name":"store_name","type":"string"},{"name":"region_cd","type":"string"},{"name":"floor_area","type":"double"}]')) PARTITIONED BY ALL"####,
    r####"REPLACE INTO "dim_region" OVERWRITE ALL SELECT TIMESTAMP '2000-01-01 00:00:00' AS "__time", "region_cd", "region_name" FROM TABLE(EXTERN('{"type":"inline","data":"{\"region_cd\":\"r1\",\"region_name\":\"north\"}\n{\"region_cd\":\"r2\",\"region_name\":\"south\"}"}', '{"type":"json"}', '[{"name":"region_cd","type":"string"},{"name":"region_name","type":"string"}]')) PARTITIONED BY ALL"####,
    r####"REPLACE INTO "receipt" OVERWRITE ALL SELECT TIMESTAMP '2000-01-01 00:00:00' AS "__time", "receipt_no", "tax" FROM TABLE(EXTERN('{"type":"inline","data":"{\"receipt_no\":\"R1\",\"tax\":1}\n{\"receipt_no\":\"R2\",\"tax\":2}\n{\"receipt_no\":\"R3\",\"tax\":5}\n{\"receipt_no\":\"R5\",\"tax\":3}\n{\"receipt_no\":\"R6\",\"tax\":9}\n{\"receipt_no\":\"R9\",\"tax\":100}"}', '{"type":"json"}', '[{"name":"receipt_no","type":"string"},{"name":"tax","type":"double"}]')) PARTITIONED BY ALL"####,
    r####"REPLACE INTO "promo" OVERWRITE ALL SELECT TIMESTAMP '2000-01-01 00:00:00' AS "__time", "promo_cd", "region_cd", "promo_name" FROM TABLE(EXTERN('{"type":"inline","data":"{\"promo_cd\":\"P1\",\"region_cd\":\"r1\",\"promo_name\":\"spring-north\"}\n{\"promo_cd\":\"P1\",\"region_cd\":\"r2\",\"promo_name\":\"spring-south\"}\n{\"promo_cd\":\"P2\",\"region_cd\":\"r2\",\"promo_name\":\"autumn-south\"}"}', '{"type":"json"}', '[{"name":"promo_cd","type":"string"},{"name":"region_cd","type":"string"},{"name":"promo_name","type":"string"}]')) PARTITIONED BY ALL"####,
    r####"REPLACE INTO "refund_dtl" OVERWRITE ALL SELECT TIMESTAMP '2000-01-01 00:00:00' AS "__time", "refund_id", "store_cd", "amount" FROM TABLE(EXTERN('{"type":"inline","data":"{\"refund_id\":1,\"store_cd\":\"s1\",\"amount\":4}\n{\"refund_id\":2,\"store_cd\":\"s1\",\"amount\":6}\n{\"refund_id\":3,\"store_cd\":\"s2\",\"amount\":5}\n{\"refund_id\":4,\"store_cd\":\"s4\",\"amount\":8}"}', '{"type":"json"}', '[{"name":"refund_id","type":"long"},{"name":"store_cd","type":"string"},{"name":"amount","type":"double"}]')) PARTITIONED BY ALL"####,
    r####"REPLACE INTO "visit" OVERWRITE ALL SELECT TIMESTAMP '2000-01-01 00:00:00' AS "__time", "visit_id", "store_cd" FROM TABLE(EXTERN('{"type":"inline","data":"{\"visit_id\":1,\"store_cd\":\"s1\"}\n{\"visit_id\":2,\"store_cd\":\"s3\"}\n{\"visit_id\":3,\"store_cd\":\"s4\"}\n{\"visit_id\":4,\"store_cd\":\"s4\"}"}', '{"type":"json"}', '[{"name":"visit_id","type":"long"},{"name":"store_cd","type":"string"}]')) PARTITIONED BY ALL"####,
];

/// The property-insurance tables.
const INSURANCE: &[&str] = &[
    r####"REPLACE INTO "ins_policy" OVERWRITE ALL SELECT "policy_id", "policy_no", "product_cd", "channel_cd", "branch_cd", "insured_id", "insured_type", "renewal_of", "status", "sum_insured", "written_premium", TIME_PARSE("issued_at") AS "__time" FROM TABLE(EXTERN('{"type":"inline","data":"{\"policy_id\":1,\"policy_no\":\"POL-001\",\"product_cd\":\"P_MOT\",\"channel_cd\":\"AG\",\"branch_cd\":\"B1\",\"insured_id\":101,\"insured_type\":\"PERSONAL\",\"renewal_of\":null,\"status\":\"EXPIRED\",\"sum_insured\":20000,\"written_premium\":1200,\"issued_at\":\"2025-02-10T00:00:00\"}\n{\"policy_id\":2,\"policy_no\":\"POL-002\",\"product_cd\":\"P_HOME\",\"channel_cd\":\"BR\",\"branch_cd\":\"B2\",\"insured_id\":102,\"insured_type\":\"PERSONAL\",\"renewal_of\":null,\"status\":\"EXPIRED\",\"sum_insured\":300000,\"written_premium\":900,\"issued_at\":\"2025-05-20T00:00:00\"}\n{\"policy_id\":3,\"policy_no\":\"POL-003\",\"product_cd\":\"P_LIAB\",\"channel_cd\":\"BR\",\"branch_cd\":\"B1\",\"insured_id\":103,\"insured_type\":\"COMMERCIAL\",\"renewal_of\":null,\"status\":\"EXPIRED\",\"sum_insured\":1000000,\"written_premium\":5000,\"issued_at\":\"2025-03-15T00:00:00\"}\n{\"policy_id\":4,\"policy_no\":\"POL-004\",\"product_cd\":\"P_MOT\",\"channel_cd\":\"AG\",\"branch_cd\":\"B1\",\"insured_id\":101,\"insured_type\":\"PERSONAL\",\"renewal_of\":1,\"status\":\"IN_FORCE\",\"sum_insured\":20000,\"written_premium\":1300,\"issued_at\":\"2026-02-10T00:00:00\"}\n{\"policy_id\":5,\"policy_no\":\"POL-005\",\"product_cd\":\"P_HOME\",\"channel_cd\":\"BR\",\"branch_cd\":\"B2\",\"insured_id\":102,\"insured_type\":\"PERSONAL\",\"renewal_of\":2,\"status\":\"IN_FORCE\",\"sum_insured\":300000,\"written_premium\":950,\"issued_at\":\"2026-05-20T00:00:00\"}\n{\"policy_id\":6,\"policy_no\":\"POL-006\",\"product_cd\":\"P_LIAB\",\"channel_cd\":\"BR\",\"branch_cd\":\"B1\",\"insured_id\":103,\"insured_type\":\"COMMERCIAL\",\"renewal_of\":3,\"status\":\"IN_FORCE\",\"sum_insured\":1000000,\"written_premium\":5500,\"issued_at\":\"2026-03-15T00:00:00\"}\n{\"policy_id\":7,\"policy_no\":\"POL-007\",\"product_cd\":\"P_TPL\",\"channel_cd\":\"DI\",\"branch_cd\":\"B3\",\"insured_id\":104,\"insured_type\":\"PERSONAL\",\"renewal_of\":null,\"status\":\"IN_FORCE\",\"sum_insured\":50000,\"written_premium\":800,\"issued_at\":\"2026-01-05T00:00:00\"}\n{\"policy_id\":8,\"policy_no\":\"POL-008\",\"product_cd\":\"P_MOT\",\"channel_cd\":\"AG\",\"branch_cd\":\"B3\",\"insured_id\":105,\"insured_type\":\"COMMERCIAL\",\"renewal_of\":null,\"status\":\"IN_FORCE\",\"sum_insured\":40000,\"written_premium\":2400,\"issued_at\":\"2026-04-01T00:00:00\"}\n{\"policy_id\":9,\"policy_no\":\"POL-009\",\"product_cd\":\"P_HOME\",\"channel_cd\":\"DI\",\"branch_cd\":\"B3\",\"insured_id\":106,\"insured_type\":\"PERSONAL\",\"renewal_of\":null,\"status\":\"CANCELLED\",\"sum_insured\":250000,\"written_premium\":850,\"issued_at\":\"2026-02-25T00:00:00\"}"}', '{"type":"json"}', '[{"name":"policy_id","type":"long"},{"name":"policy_no","type":"string"},{"name":"product_cd","type":"string"},{"name":"channel_cd","type":"string"},{"name":"branch_cd","type":"string"},{"name":"insured_id","type":"long"},{"name":"insured_type","type":"string"},{"name":"renewal_of","type":"long"},{"name":"status","type":"string"},{"name":"sum_insured","type":"double"},{"name":"written_premium","type":"double"},{"name":"issued_at","type":"string"}]')) PARTITIONED BY ALL"####,
    r####"REPLACE INTO "ins_earned_premium" OVERWRITE ALL SELECT "earned_id", "policy_id", TIME_PARSE("earned_month") AS "__time", "earned_premium", "exposure" FROM TABLE(EXTERN('{"type":"inline","data":"{\"earned_id\":1,\"policy_id\":1,\"earned_month\":\"2025-12-01T00:00:00\",\"earned_premium\":100,\"exposure\":1}\n{\"earned_id\":2,\"policy_id\":1,\"earned_month\":\"2026-01-01T00:00:00\",\"earned_premium\":100,\"exposure\":1}\n{\"earned_id\":3,\"policy_id\":4,\"earned_month\":\"2026-03-01T00:00:00\",\"earned_premium\":110,\"exposure\":1}\n{\"earned_id\":4,\"policy_id\":4,\"earned_month\":\"2026-04-01T00:00:00\",\"earned_premium\":110,\"exposure\":1}\n{\"earned_id\":5,\"policy_id\":5,\"earned_month\":\"2026-06-01T00:00:00\",\"earned_premium\":80,\"exposure\":1}\n{\"earned_id\":6,\"policy_id\":6,\"earned_month\":\"2026-04-01T00:00:00\",\"earned_premium\":460,\"exposure\":1}\n{\"earned_id\":7,\"policy_id\":6,\"earned_month\":\"2026-05-01T00:00:00\",\"earned_premium\":460,\"exposure\":1}\n{\"earned_id\":8,\"policy_id\":7,\"earned_month\":\"2026-02-01T00:00:00\",\"earned_premium\":70,\"exposure\":1}\n{\"earned_id\":9,\"policy_id\":7,\"earned_month\":\"2026-03-01T00:00:00\",\"earned_premium\":70,\"exposure\":1}\n{\"earned_id\":10,\"policy_id\":8,\"earned_month\":\"2026-05-01T00:00:00\",\"earned_premium\":200,\"exposure\":2}\n{\"earned_id\":11,\"policy_id\":8,\"earned_month\":\"2026-06-01T00:00:00\",\"earned_premium\":200,\"exposure\":2}\n{\"earned_id\":12,\"policy_id\":3,\"earned_month\":\"2026-01-01T00:00:00\",\"earned_premium\":420,\"exposure\":1}"}', '{"type":"json"}', '[{"name":"earned_id","type":"long"},{"name":"policy_id","type":"long"},{"name":"earned_month","type":"string"},{"name":"earned_premium","type":"double"},{"name":"exposure","type":"double"}]')) PARTITIONED BY ALL"####,
    r####"REPLACE INTO "ins_claim" OVERWRITE ALL SELECT "claim_id", "claim_no", "policy_id", "cause_cd", "status", TIME_PARSE("loss_at") AS "__time", "reported_at", "report_lag_days", "is_large_loss" FROM TABLE(EXTERN('{"type":"inline","data":"{\"claim_id\":1,\"claim_no\":\"CLM-001\",\"policy_id\":1,\"cause_cd\":\"COLLISION\",\"status\":\"CLOSED\",\"loss_at\":\"2025-06-10T00:00:00\",\"reported_at\":\"2025-06-12T00:00:00\",\"report_lag_days\":2,\"is_large_loss\":0}\n{\"claim_id\":2,\"claim_no\":\"CLM-002\",\"policy_id\":3,\"cause_cd\":\"INJURY\",\"status\":\"OPEN\",\"loss_at\":\"2025-11-20T00:00:00\",\"reported_at\":\"2025-12-05T00:00:00\",\"report_lag_days\":15,\"is_large_loss\":1}\n{\"claim_id\":3,\"claim_no\":\"CLM-003\",\"policy_id\":4,\"cause_cd\":\"COLLISION\",\"status\":\"CLOSED\",\"loss_at\":\"2026-03-03T00:00:00\",\"reported_at\":\"2026-03-04T00:00:00\",\"report_lag_days\":1,\"is_large_loss\":0}\n{\"claim_id\":4,\"claim_no\":\"CLM-004\",\"policy_id\":4,\"cause_cd\":\"THEFT\",\"status\":\"OPEN\",\"loss_at\":\"2026-04-18T00:00:00\",\"reported_at\":\"2026-04-21T00:00:00\",\"report_lag_days\":3,\"is_large_loss\":1}\n{\"claim_id\":5,\"claim_no\":\"CLM-005\",\"policy_id\":5,\"cause_cd\":\"WATER\",\"status\":\"OPEN\",\"loss_at\":\"2026-06-02T00:00:00\",\"reported_at\":\"2026-06-09T00:00:00\",\"report_lag_days\":7,\"is_large_loss\":0}\n{\"claim_id\":6,\"claim_no\":\"CLM-006\",\"policy_id\":8,\"cause_cd\":\"COLLISION\",\"status\":\"OPEN\",\"loss_at\":\"2026-05-15T00:00:00\",\"reported_at\":\"2026-05-15T00:00:00\",\"report_lag_days\":0,\"is_large_loss\":0}"}', '{"type":"json"}', '[{"name":"claim_id","type":"long"},{"name":"claim_no","type":"string"},{"name":"policy_id","type":"long"},{"name":"cause_cd","type":"string"},{"name":"status","type":"string"},{"name":"loss_at","type":"string"},{"name":"reported_at","type":"string"},{"name":"report_lag_days","type":"long"},{"name":"is_large_loss","type":"long"}]')) PARTITIONED BY ALL"####,
    r####"REPLACE INTO "ins_claim_reserve" OVERWRITE ALL SELECT TIMESTAMP '2000-01-01 00:00:00' AS "__time", "claim_id", "outstanding" FROM TABLE(EXTERN('{"type":"inline","data":"{\"claim_id\":2,\"outstanding\":8000}\n{\"claim_id\":4,\"outstanding\":15000}\n{\"claim_id\":5,\"outstanding\":600}\n{\"claim_id\":6,\"outstanding\":900}"}', '{"type":"json"}', '[{"name":"claim_id","type":"long"},{"name":"outstanding","type":"double"}]')) PARTITIONED BY ALL"####,
    r####"REPLACE INTO "ins_claim_payment" OVERWRITE ALL SELECT "payment_id", "claim_id", "payment_type", TIME_PARSE("paid_at") AS "__time", "paid_amount" FROM TABLE(EXTERN('{"type":"inline","data":"{\"payment_id\":1,\"claim_id\":1,\"payment_type\":\"INDEMNITY\",\"paid_at\":\"2025-07-01T00:00:00\",\"paid_amount\":700}\n{\"payment_id\":2,\"claim_id\":1,\"payment_type\":\"EXPENSE\",\"paid_at\":\"2025-07-15T00:00:00\",\"paid_amount\":50}\n{\"payment_id\":3,\"claim_id\":2,\"payment_type\":\"INDEMNITY\",\"paid_at\":\"2026-01-20T00:00:00\",\"paid_amount\":3000}\n{\"payment_id\":4,\"claim_id\":2,\"payment_type\":\"EXPENSE\",\"paid_at\":\"2026-02-10T00:00:00\",\"paid_amount\":400}\n{\"payment_id\":5,\"claim_id\":3,\"payment_type\":\"INDEMNITY\",\"paid_at\":\"2026-03-20T00:00:00\",\"paid_amount\":500}\n{\"payment_id\":6,\"claim_id\":4,\"payment_type\":\"INDEMNITY\",\"paid_at\":\"2026-05-05T00:00:00\",\"paid_amount\":4000}\n{\"payment_id\":7,\"claim_id\":4,\"payment_type\":\"EXPENSE\",\"paid_at\":\"2026-06-10T00:00:00\",\"paid_amount\":250}\n{\"payment_id\":8,\"claim_id\":5,\"payment_type\":\"INDEMNITY\",\"paid_at\":\"2026-06-20T00:00:00\",\"paid_amount\":300}"}', '{"type":"json"}', '[{"name":"payment_id","type":"long"},{"name":"claim_id","type":"long"},{"name":"payment_type","type":"string"},{"name":"paid_at","type":"string"},{"name":"paid_amount","type":"double"}]')) PARTITIONED BY ALL"####,
    r####"REPLACE INTO "ins_cession" OVERWRITE ALL SELECT TIMESTAMP '2000-01-01 00:00:00' AS "__time", "cession_id", "policy_id", "treaty_cd", "ceded_premium" FROM TABLE(EXTERN('{"type":"inline","data":"{\"cession_id\":1,\"policy_id\":3,\"treaty_cd\":\"QS\",\"ceded_premium\":1500}\n{\"cession_id\":2,\"policy_id\":6,\"treaty_cd\":\"QS\",\"ceded_premium\":1650}\n{\"cession_id\":3,\"policy_id\":6,\"treaty_cd\":\"XL\",\"ceded_premium\":300}\n{\"cession_id\":4,\"policy_id\":8,\"treaty_cd\":\"QS\",\"ceded_premium\":720}\n{\"cession_id\":5,\"policy_id\":4,\"treaty_cd\":\"QS\",\"ceded_premium\":390}"}', '{"type":"json"}', '[{"name":"cession_id","type":"long"},{"name":"policy_id","type":"long"},{"name":"treaty_cd","type":"string"},{"name":"ceded_premium","type":"double"}]')) PARTITIONED BY ALL"####,
    r####"REPLACE INTO "ins_product" OVERWRITE ALL SELECT TIMESTAMP '2000-01-01 00:00:00' AS "__time", "product_cd", "product_name", "line_of_business" FROM TABLE(EXTERN('{"type":"inline","data":"{\"product_cd\":\"P_MOT\",\"product_name\":\"Motor Own Damage\",\"line_of_business\":\"MOTOR\"}\n{\"product_cd\":\"P_TPL\",\"product_name\":\"Motor Liability\",\"line_of_business\":\"MOTOR\"}\n{\"product_cd\":\"P_HOME\",\"product_name\":\"Home\",\"line_of_business\":\"PROPERTY\"}\n{\"product_cd\":\"P_LIAB\",\"product_name\":\"Public Liability\",\"line_of_business\":\"LIABILITY\"}"}', '{"type":"json"}', '[{"name":"product_cd","type":"string"},{"name":"product_name","type":"string"},{"name":"line_of_business","type":"string"}]')) PARTITIONED BY ALL"####,
    r####"REPLACE INTO "ins_channel" OVERWRITE ALL SELECT TIMESTAMP '2000-01-01 00:00:00' AS "__time", "channel_cd", "channel_name" FROM TABLE(EXTERN('{"type":"inline","data":"{\"channel_cd\":\"AG\",\"channel_name\":\"Agency\"}\n{\"channel_cd\":\"BR\",\"channel_name\":\"Broker\"}\n{\"channel_cd\":\"DI\",\"channel_name\":\"Direct\"}"}', '{"type":"json"}', '[{"name":"channel_cd","type":"string"},{"name":"channel_name","type":"string"}]')) PARTITIONED BY ALL"####,
    r####"REPLACE INTO "ins_branch" OVERWRITE ALL SELECT TIMESTAMP '2000-01-01 00:00:00' AS "__time", "branch_cd", "branch_name", "region_name" FROM TABLE(EXTERN('{"type":"inline","data":"{\"branch_cd\":\"B1\",\"branch_name\":\"East Branch\",\"region_name\":\"East\"}\n{\"branch_cd\":\"B2\",\"branch_name\":\"North Branch\",\"region_name\":\"North\"}\n{\"branch_cd\":\"B3\",\"branch_name\":\"Lake Branch\",\"region_name\":\"North\"}"}', '{"type":"json"}', '[{"name":"branch_cd","type":"string"},{"name":"branch_name","type":"string"},{"name":"region_name","type":"string"}]')) PARTITIONED BY ALL"####,
];

pub fn fixture(fixture: Fixture) -> &'static [&'static str] {
    match fixture {
        Fixture::TokyoOctober => TOKYO_OCTOBER,
        Fixture::NewYorkDst => NEW_YORK_DST,
        Fixture::Retail => RETAIL,
        Fixture::Insurance => INSURANCE,
    }
}

// scenario: B0-DRUID-003
// Compared with pinned Cube Core by a Druid execution tier, not by T0.
/// How Cube Core's Druid dialect answers a case, where not with the kernel's
/// SQL. `refuses`: it has no template for the feature and returns an error.
/// `differs`: its SQL is other than the kernel's and Druid 26 rejects it: an
/// interval written `INTERVAL 3 day`, or several SELECT statements merged through a
/// null-safe join over a union of keys. A Druid execution tier holds each to
/// its word; a case not listed here must compile to Cube Core's SQL.
pub fn cube_core(case: &str) -> Option<&'static str> {
    match case {
        "approximate_distinct_count"
        | "granularity_of_two_days"
        | "granularity_of_two_quarters" => Some("refuses"),
        // An interval literal Druid cannot parse.
        "granularity_with_offset"
        | "uw_by_underwriting_year"
        | "prior_day"
        | "change_from_prior_day" => Some("differs"),
        // Several SELECT statements over shared dimensions. Totals are not here: without
        // a dimension Cube joins them `ON 1 = 1`, as the kernel does.
        "multi_fact_by_dimension"
        | "multi_fact_null_key"
        | "multi_fact_page"
        | "sum_over_repeated_rows"
        | "security_multi_fact"
        | "view_sum_over_repeated_rows"
        | "view_formula_and_one_to_one"
        | "ep_written_vs_earned_by_line"
        | "cl_paid_and_outstanding_by_line"
        | "x_premium_paid_by_line"
        | "x_premium_paid_ceded_by_region"
        | "security_region_across_facts" => Some("differs"),
        _ => None,
    }
}

// scenario: B0-DRUID-004
// Executed on a real engine by a Druid execution tier, not by T0.
pub fn scenario(_group: Group) -> &'static str {
    "B0-DRUID-004"
}
