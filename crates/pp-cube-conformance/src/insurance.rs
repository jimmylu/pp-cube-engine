//! A property-insurance scenario: business queries on the model in
//! `fixtures/insurance.yaml`.
//!
//! `survey` is every query an analyst of the domain would ask that was put to
//! the kernel, whether or not it compiles; the manifest test records which do.
//! `cases` are the ones that compile and have an expected result, computed from
//! the fixture rows by hand, not from SQL.

use serde_json::Value;

use crate::cases::{Declared, Fixture, Group};

/// Name and Cube query of every surveyed question, in the order of the matrix.
pub fn survey() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "uw_premium_by_line_channel",
            r#"{"measures":["policies.written_premium","policies.policy_count","policies.avg_premium"],"dimensions":["products.line_of_business","channels.channel_name"],"order":[["products.line_of_business","asc"],["channels.channel_name","asc"]]}"#,
        ),
        (
            "uw_renewal_share_by_branch",
            r#"{"measures":["policies.renewal_share","policies.renewed_count","policies.policy_count"],"dimensions":["branches.branch_name"],"order":[["branches.branch_name","asc"]]}"#,
        ),
        (
            "uw_rate_in_force_by_product",
            r#"{"measures":["policies.rate_per_mille","policies.sum_insured"],"dimensions":["products.product_name"],"segments":["policies.in_force"],"order":[["products.product_name","asc"]]}"#,
        ),
        (
            "uw_premium_ytd_by_month",
            r#"{"measures":["policies.written_premium","policies.written_premium_ytd"],"timeDimensions":[{"dimension":"policies.issued_at","granularity":"month","dateRange":["2026-01-01","2026-06-30"]}],"order":[["policies.issued_at","asc"]]}"#,
        ),
        (
            "uw_premium_qtd_by_month",
            r#"{"measures":["policies.written_premium_qtd"],"timeDimensions":[{"dimension":"policies.issued_at","granularity":"month","dateRange":["2026-01-01","2026-06-30"]}],"order":[["policies.issued_at","asc"]]}"#,
        ),
        (
            "uw_premium_ytd_by_quarter",
            r#"{"measures":["policies.written_premium_ytd"],"timeDimensions":[{"dimension":"policies.issued_at","granularity":"quarter","dateRange":["2026-01-01","2026-12-31"]}]}"#,
        ),
        (
            "uw_premium_vs_prior_year",
            r#"{"measures":["policies.written_premium","policies.written_premium_prior_year","policies.written_premium_growth"],"dimensions":["products.line_of_business"],"timeDimensions":[{"dimension":"policies.issued_at","granularity":"year","dateRange":["2026-01-01","2026-12-31"]}],"order":[["products.line_of_business","asc"]]}"#,
        ),
        (
            "uw_by_underwriting_year",
            r#"{"measures":["policies.written_premium"],"timeDimensions":[{"dimension":"policies.issued_at","granularity":"underwriting_year","dateRange":["2025-01-01","2026-12-31"]}],"order":[["policies.issued_at","asc"]]}"#,
        ),
        (
            "uw_insureds_exact_and_approx",
            r#"{"measures":["policies.insured_count","policies.insured_count_approx"],"dimensions":["policies.insured_type"],"order":[["policies.insured_type","asc"]]}"#,
        ),
        (
            "uw_top_branches",
            r#"{"measures":["policies.written_premium"],"dimensions":["branches.branch_name"],"filters":[{"member":"policies.written_premium","operator":"gt","values":["1000"]}],"order":[["policies.written_premium","desc"]],"limit":2}"#,
        ),
        (
            "uw_policy_listing",
            r#"{"ungrouped":true,"dimensions":["policies.policy_no","products.product_name","branches.branch_name"],"measures":["policies.written_premium"],"filters":[{"member":"policies.is_renewal","operator":"equals","values":["true"]}],"order":[["policies.policy_no","asc"]],"limit":50}"#,
        ),
        (
            "uw_last_12_months_relative",
            r#"{"measures":["policies.written_premium"],"timeDimensions":[{"dimension":"policies.issued_at","granularity":"month","dateRange":"last 12 months"}]}"#,
        ),
        (
            "uw_compare_years",
            r#"{"measures":["policies.written_premium"],"timeDimensions":[{"dimension":"policies.issued_at","compareDateRange":[["2025-01-01","2025-12-31"],["2026-01-01","2026-12-31"]]}]}"#,
        ),
        (
            "ep_earned_and_exposure_by_line",
            r#"{"measures":["earned_premiums.earned_premium","earned_premiums.exposure"],"dimensions":["products.line_of_business"],"timeDimensions":[{"dimension":"earned_premiums.earned_month","dateRange":["2026-01-01","2026-06-30"]}],"order":[["products.line_of_business","asc"]]}"#,
        ),
        (
            "ep_rolling_12m_by_month",
            r#"{"measures":["earned_premiums.earned_premium_12m"],"timeDimensions":[{"dimension":"earned_premiums.earned_month","granularity":"month","dateRange":["2026-01-01","2026-06-30"]}],"order":[["earned_premiums.earned_month","asc"]]}"#,
        ),
        (
            "ep_written_vs_earned_by_line",
            r#"{"measures":["policies.written_premium","earned_premiums.earned_premium"],"dimensions":["products.line_of_business"],"order":[["products.line_of_business","asc"]]}"#,
        ),
        (
            "cl_counts_by_cause",
            r#"{"measures":["claims.claim_count","claims.open_count","claims.avg_report_lag_days","claims.max_report_lag_days"],"dimensions":["claims.cause_cd"],"order":[["claims.cause_cd","asc"]]}"#,
        ),
        (
            "cl_outstanding_by_line",
            r#"{"measures":["claims.claim_count","claim_reserves.outstanding","claims.avg_outstanding"],"dimensions":["products.line_of_business"],"order":[["products.line_of_business","asc"]]}"#,
        ),
        (
            "cl_paid_split_by_line",
            r#"{"measures":["claim_payments.paid_amount","claim_payments.indemnity_paid","claim_payments.expense_paid","claim_payments.paid_per_claim"],"dimensions":["products.line_of_business"],"order":[["products.line_of_business","asc"]]}"#,
        ),
        (
            "cl_paid_and_outstanding_by_line",
            r#"{"measures":["claim_payments.paid_amount","claim_reserves.outstanding"],"dimensions":["products.line_of_business"],"order":[["products.line_of_business","asc"]]}"#,
        ),
        (
            "cl_development_triangle",
            r#"{"measures":["claim_payments.paid_amount"],"timeDimensions":[{"dimension":"claims.loss_at","granularity":"year"},{"dimension":"claim_payments.paid_at","granularity":"year"}],"order":[["claims.loss_at","asc"],["claim_payments.paid_at","asc"]]}"#,
        ),
        (
            "cl_paid_ytd_by_month",
            r#"{"measures":["claim_payments.paid_amount_ytd"],"timeDimensions":[{"dimension":"claim_payments.paid_at","granularity":"month","dateRange":["2026-01-01","2026-06-30"]}],"order":[["claim_payments.paid_at","asc"]]}"#,
        ),
        (
            "cl_large_loss_listing",
            r#"{"ungrouped":true,"dimensions":["claims.claim_no","claims.cause_cd","policies.policy_no","products.line_of_business"],"segments":["claims.large_loss"],"timeDimensions":[{"dimension":"claims.loss_at","granularity":"day","dateRange":["2026-01-01","2026-12-31"]}],"order":[["claims.claim_no","asc"]]}"#,
        ),
        (
            "cl_reported_by_hour",
            r#"{"measures":["claims.claim_count"],"timeDimensions":[{"dimension":"claims.reported_at","granularity":"hour","dateRange":["2026-03-01","2026-03-01"]}]}"#,
        ),
        (
            "cl_lines_with_many_claims",
            r#"{"measures":["claim_payments.paid_amount"],"dimensions":["products.line_of_business"],"filters":[{"member":"claims.claim_count","operator":"gt","values":["1"]}]}"#,
        ),
        (
            "x_premium_paid_by_line",
            r#"{"measures":["earned_premiums.earned_premium","claim_payments.paid_amount","claims.claim_count"],"dimensions":["products.line_of_business"],"order":[["products.line_of_business","asc"]]}"#,
        ),
        (
            "x_premium_paid_ceded_by_region",
            r#"{"measures":["policies.written_premium","cessions.ceded_premium","claim_payments.paid_amount"],"dimensions":["branches.region_name"],"filters":[{"member":"products.line_of_business","operator":"equals","values":["MOTOR"]}],"order":[["branches.region_name","asc"]]}"#,
        ),
        (
            "x_loss_ratio_measure",
            r#"{"measures":["earned_premiums.paid_loss_ratio"],"dimensions":["products.line_of_business"]}"#,
        ),
        (
            "x_earned_and_paid_by_month",
            r#"{"measures":["earned_premiums.earned_premium","claim_payments.paid_amount"],"timeDimensions":[{"dimension":"earned_premiums.earned_month","granularity":"month","dateRange":["2026-01-01","2026-06-30"]}]}"#,
        ),
        (
            "x_totals",
            r#"{"measures":["policies.written_premium","earned_premiums.earned_premium","claim_payments.paid_amount","claim_reserves.outstanding","cessions.ceded_premium"]}"#,
        ),
        (
            "v_underwriting_by_region",
            r#"{"measures":["underwriting.written_premium","underwriting.policy_count","underwriting.renewal_share"],"dimensions":["underwriting.region_name","underwriting.line_of_business"],"order":[["underwriting.region_name","asc"],["underwriting.line_of_business","asc"]]}"#,
        ),
        (
            "v_underwriting_ytd",
            r#"{"measures":["underwriting.written_premium_ytd"],"dimensions":["underwriting.channel_name"],"timeDimensions":[{"dimension":"underwriting.issued_at","granularity":"month","dateRange":["2026-01-01","2026-06-30"]}],"order":[["underwriting.channel_name","asc"],["underwriting.issued_at","asc"]]}"#,
        ),
        (
            "v_claims_by_line_cause",
            r#"{"measures":["claims_overview.paid_amount","claims_overview.claim_count","claims_overview.paid_per_claim"],"dimensions":["claims_overview.line_of_business","claims_overview.cause_cd"],"timeDimensions":[{"dimension":"claims_overview.paid_at","dateRange":["2026-01-01","2026-12-31"]}],"order":[["claims_overview.line_of_business","asc"],["claims_overview.cause_cd","asc"]]}"#,
        ),
    ]
}

fn query(name: &str) -> String {
    let found = survey().into_iter().find(|(known, _)| *known == name);
    found.map(|(_, query)| query.to_owned()).unwrap_or_default()
}

fn case(name: &'static str, columns: &[&str], rows: &str) -> Declared {
    let rows: Vec<Vec<Value>> = serde_json::from_str(rows).unwrap_or_default();
    Declared {
        name,
        group: Group::Domain,
        fixture: Fixture::Insurance,
        query: query(name),
        timezone: "UTC",
        security: "",
        columns: columns.iter().map(|column| (*column).into()).collect(),
        rows,
    }
}

/// The surveyed queries that compile and are executed.
pub fn cases() -> Vec<Declared> {
    let mut cases = vec![
        // Renewals are terms that name the term they renew: two of four in the east.
        case(
            "uw_renewal_share_by_branch",
            &[
                "branches__branch_name",
                "policies__renewal_share",
                "policies__renewed_count",
                "policies__policy_count",
            ],
            r#"[["East Branch",0.5,2,4],["Lake Branch",0.0,0,3],["North Branch",0.5,1,2]]"#,
        ),
        // June has no new policy: nothing written, and the year to date stands.
        case(
            "uw_premium_ytd_by_month",
            &[
                "policies__issued_at_month",
                "policies__written_premium",
                "policies__written_premium_ytd",
            ],
            r#"[[{"timestamp":"2026-01-01T00:00:00"},800,800],[{"timestamp":"2026-02-01T00:00:00"},2150,2950],[{"timestamp":"2026-03-01T00:00:00"},5500,8450],[{"timestamp":"2026-04-01T00:00:00"},2400,10850],[{"timestamp":"2026-05-01T00:00:00"},950,11800],[{"timestamp":"2026-06-01T00:00:00"},null,11800]]"#,
        ),
        // The underwriting year starts in April: January to March belong to the year before.
        case(
            "uw_by_underwriting_year",
            &[
                "policies__issued_at_underwriting_year",
                "policies__written_premium",
            ],
            r#"[[{"timestamp":"2024-04-01T00:00:00"},6200],[{"timestamp":"2025-04-01T00:00:00"},9350],[{"timestamp":"2026-04-01T00:00:00"},3350]]"#,
        ),
        // Rows, not totals: the three renewed terms.
        case(
            "uw_policy_listing",
            &[
                "policies__policy_no",
                "products__product_name",
                "branches__branch_name",
                "policies__written_premium",
            ],
            r#"[["POL-004","Motor Own Damage","East Branch",1300],["POL-005","Home","North Branch",950],["POL-006","Public Liability","East Branch",5500]]"#,
        ),
        // Twelve months ending with each month; December of last year leaves no window here.
        case(
            "ep_rolling_12m_by_month",
            &[
                "earned_premiums__earned_month_month",
                "earned_premiums__earned_premium_12m",
            ],
            r#"[[{"timestamp":"2026-01-01T00:00:00"},620],[{"timestamp":"2026-02-01T00:00:00"},690],[{"timestamp":"2026-03-01T00:00:00"},870],[{"timestamp":"2026-04-01T00:00:00"},1440],[{"timestamp":"2026-05-01T00:00:00"},2100],[{"timestamp":"2026-06-01T00:00:00"},2380]]"#,
        ),
        // Two facts, each totalled by line on its own.
        case(
            "ep_written_vs_earned_by_line",
            &[
                "products__line_of_business",
                "policies__written_premium",
                "earned_premiums__earned_premium",
            ],
            r#"[["LIABILITY",10500,1340],["MOTOR",5700,960],["PROPERTY",2700,80]]"#,
        ),
        // One collision of three is still open.
        case(
            "cl_counts_by_cause",
            &[
                "claims__cause_cd",
                "claims__claim_count",
                "claims__open_count",
                "claims__avg_report_lag_days",
                "claims__max_report_lag_days",
            ],
            r#"[["COLLISION",3,1,1.0,2],["INJURY",1,1,15.0,15],["THEFT",1,1,3.0,3],["WATER",1,1,7.0,7]]"#,
        ),
        // Every claim with its reserve: one row each, so one SELECT. Two motor
        // claims are closed and hold nothing.
        case(
            "cl_outstanding_by_line",
            &[
                "products__line_of_business",
                "claims__claim_count",
                "claim_reserves__outstanding",
                "claims__avg_outstanding",
            ],
            r#"[["LIABILITY",1,8000,8000.0],["MOTOR",4,15900,3975.0],["PROPERTY",1,600,600.0]]"#,
        ),
        // Counted from payments, a reserve belongs to a claim that was paid on: the
        // open motor claim without a payment, and its 900, are not here.
        case(
            "cl_paid_and_outstanding_by_line",
            &[
                "products__line_of_business",
                "claim_payments__paid_amount",
                "claim_reserves__outstanding",
            ],
            r#"[["LIABILITY",3400,8000],["MOTOR",5500,15000],["PROPERTY",300,600]]"#,
        ),
        // Paid by loss year and payment year: last year's injury claim is paid this year.
        case(
            "cl_development_triangle",
            &[
                "claims__loss_at_year",
                "claim_payments__paid_at_year",
                "claim_payments__paid_amount",
            ],
            r#"[[{"timestamp":"2025-01-01T00:00:00"},{"timestamp":"2025-01-01T00:00:00"},750],[{"timestamp":"2025-01-01T00:00:00"},{"timestamp":"2026-01-01T00:00:00"},3400],[{"timestamp":"2026-01-01T00:00:00"},{"timestamp":"2026-01-01T00:00:00"},5050]]"#,
        ),
        // Three facts by line. The motor claim without a payment is counted, and adds nothing paid.
        case(
            "x_premium_paid_by_line",
            &[
                "products__line_of_business",
                "earned_premiums__earned_premium",
                "claim_payments__paid_amount",
                "claims__claim_count",
            ],
            r#"[["LIABILITY",1340,3400,1],["MOTOR",960,5500,4],["PROPERTY",80,300,1]]"#,
        ),
        // Three facts by region, motor only: the north has ceded premium and no payment.
        case(
            "x_premium_paid_ceded_by_region",
            &[
                "branches__region_name",
                "policies__written_premium",
                "cessions__ceded_premium",
                "claim_payments__paid_amount",
            ],
            r#"[["East",2500,390,5500],["North",3200,720,null]]"#,
        ),
        // Five facts without a dimension, side by side.
        case(
            "x_totals",
            &[
                "policies__written_premium",
                "earned_premiums__earned_premium",
                "claim_payments__paid_amount",
                "claim_reserves__outstanding",
                "cessions__ceded_premium",
            ],
            r#"[[18900,2380,9200,24500,4560]]"#,
        ),
        // Through the claims view, payments of this year only.
        case(
            "v_claims_by_line_cause",
            &[
                "claims_overview__line_of_business",
                "claims_overview__cause_cd",
                "claims_overview__paid_amount",
                "claims_overview__claim_count",
                "claims_overview__paid_per_claim",
            ],
            r#"[["LIABILITY","INJURY",3400,1,3400.0],["MOTOR","COLLISION",500,1,500.0],["MOTOR","THEFT",4250,1,4250.0],["PROPERTY","WATER",300,1,300.0]]"#,
        ),
    ];
    // A caller restricted to the east sees its part of every fact: the northern
    // motor premium and claim are gone, and property with them.
    let (columns, rows) = (
        &[
            "products__line_of_business",
            "earned_premiums__earned_premium",
            "claim_payments__paid_amount",
            "claims__claim_count",
        ],
        r#"[["LIABILITY",1340,3400,1],["MOTOR",420,5500,3]]"#,
    );
    cases.push(Declared {
        query: query("x_premium_paid_by_line"),
        security: r#"{"filters":[{"member":"branches.region_name","operator":"equals","values":["East"]}]}"#,
        ..case("security_region_across_facts", columns, rows)
    });
    cases
}
