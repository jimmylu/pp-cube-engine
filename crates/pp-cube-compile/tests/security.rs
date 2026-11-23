//! Caller restrictions through the public facade; no SQL is executed.
mod support;
use support::compile;

use std::sync::Arc;

use pp_cube_compile::{CompileContext, CompileError, Compiled, DialectId, Parameter};
use pp_cube_model::{load_query, load_yaml, Filter};

fn filters(json: &str) -> Vec<Filter> {
    load_query(&format!(r#"{{"filters": {json}}}"#))
        .expect("filters load")
        .filters
}

fn context(json: &str) -> CompileContext {
    CompileContext {
        request_id: String::new(),
        dialect: DialectId::Presto,
        timezone: "UTC".to_owned(),
        security_filters: filters(json),
    }
}

fn run(model: &str, query: &str, security: &str) -> Result<Compiled, CompileError> {
    compile(
        &Arc::new(load_yaml(model).expect("model loads")),
        &load_query(query).expect("query loads"),
        &context(security),
    )
}

const SALES: &str = include_str!("fixtures/sales.yaml");
const RETAIL: &str = include_str!("fixtures/retail.yaml");
const QUERY: &str = r#"{"measures":["sales.amount"]}"#;

// scenario: B0-SEC-001
// scenario: B0-DET-001
#[test]
fn restrictions_are_conjoined_without_mutating_inputs() {
    let model = Arc::new(load_yaml(SALES).unwrap());
    let query = load_query(
        r#"{"measures":["sales.amount"],"filters":[{"or":[
          {"member":"sales.channel_cd","operator":"equals","values":["C1"]},
          {"and":[{"member":"sales.channel_cd","operator":"equals","values":["C2"]},
                   {"member":"sales.qty","operator":"gt","values":["10"]}]}]}]}"#,
    )
    .unwrap();
    let context = context(
        r#"[
        {"member":"sales.channel_cd","operator":"equals","values":["C9"]},
        {"member":"sales.qty","operator":"lt","values":["20"]}
    ]"#,
    );
    let original = (query.clone(), context.clone());
    let result = compile(&model, &query, &context).unwrap();
    assert_eq!((query, context), original);
    assert_eq!(
        result.parameters,
        ["C1", "C2", "10", "C9", "20"].map(|s| Parameter::Text(s.to_owned()))
    );
    insta::assert_snapshot!(format!(
        "{}\n-- parameters: {:?}",
        result.sql, result.parameters
    ));
}

// scenario: B0-SEC-004
#[test]
fn empty_security_groups_fail_at_every_depth() {
    for group in [r#"{"and":[]}"#, r#"{"or":[]}"#] {
        for security in [
            format!("[{group}]"),
            format!(r#"[{{"and":[{{"member":"sales.qty","operator":"set"}},{group}]}}]"#),
            format!(r#"[{{"or":[{{"member":"sales.qty","operator":"set"}},{group}]}}]"#),
        ] {
            assert_eq!(
                run(SALES, QUERY, &security),
                Err(CompileError::Query(
                    "A security filter group must not be empty".to_owned()
                )),
                "{security}"
            );
        }
    }
}

// scenario: B0-SEC-004
// scenario: B0-DET-001
#[test]
fn empty_context_preserves_the_original_query_output() {
    let plain = run(SALES, QUERY, "[]").unwrap();
    insta::assert_snapshot!(plain.sql, @r#"
    SELECT sum("sales".amount) "sales__amount"  
    FROM  dw.sales_dtl  AS "sales"
    "#);
    assert!(plain.parameters.is_empty());
}

// scenario: B0-SEC-001
// scenario: B0-DET-001
#[test]
fn user_or_cannot_bypass_region_restriction() {
    let result = run(
        SALES,
        r#"{"measures":["sales.amount"],"filters":[{"or":[
        {"member":"sales.channel_cd","operator":"equals","values":["C1"]},
        {"member":"sales.category_cd","operator":"equals","values":["TOY"]}
    ]}]}"#,
        r#"[{"member":"sales.region_cd","operator":"equals","values":["R1"]}]"#,
    )
    .unwrap();
    assert_eq!(
        result.parameters,
        ["C1", "TOY", "R1"].map(|v| Parameter::Text(v.to_owned()))
    );
    insta::assert_snapshot!(format!(
        "{}\n-- parameters: {:?}",
        result.sql, result.parameters
    ));
}

// scenario: B0-SEC-002
// scenario: B0-DET-001
#[test]
fn mandatory_cube_predicate_survives_without_and_with_other_filters() {
    let mut outputs = Vec::new();
    for (query, security) in [
        (r#"{"measures":["valid_sales.amount"]}"#, "[]"),
        (
            r#"{"measures":["valid_sales.amount"],"filters":[{"member":"valid_sales.channel_cd","operator":"notEquals","values":["C9"]}]}"#,
            r#"[{"member":"valid_sales.channel_cd","operator":"equals","values":["C1"]}]"#,
        ),
    ] {
        let result = run(SALES, query, security).unwrap();
        assert!(result.sql.contains("WHERE is_valid = 1"), "{}", result.sql);
        outputs.push(format!(
            "{}\n-- parameters: {:?}",
            result.sql, result.parameters
        ));
    }
    insta::assert_snapshot!(outputs.join("\n\n"));
}

// scenario: B0-SEC-003
#[test]
fn malformed_security_filters_cannot_be_deserialized() {
    for filter in [
        r#"{"member":"sales.qty"}"#,
        r#"{"member":"sales.qty","operator":"unknown"}"#,
        r#"{"member":"sales.qty","operator":"set","and":[]}"#,
    ] {
        assert!(load_query(&format!(r#"{{"filters":[{filter}]}}"#)).is_err());
    }
}

// scenario: B0-SEC-003
#[test]
fn invalid_security_conditions_fail_without_sql() {
    for (condition, message) in [
        (
            r#"{"member":"sales.missing","operator":"set"}"#,
            "sales.missing",
        ),
        (
            r#"{"member":"sales.qty","operator":"contains","values":["1"]}"#,
            "does not fit",
        ),
        (
            r#"{"member":"sales.qty","operator":"gt"}"#,
            "needs at least one",
        ),
        (
            r#"{"member":"sales.qty","operator":"set","values":["1"]}"#,
            "takes no values",
        ),
        (
            r#"{"member":"sales.sold_at","operator":"inDateRange","values":["2026-10-01"]}"#,
            "takes two dates",
        ),
        (
            r#"{"member":"sales.sold_at","operator":"beforeDate","values":["bad"]}"#,
            "not a date",
        ),
    ] {
        let result = run(SALES, QUERY, &format!("[{condition}]"));
        assert!(
            matches!(&result, Err(CompileError::Query(text)) if text.contains(message)),
            "{result:?}"
        );
    }
}

// scenario: B0-SEC-003
// scenario: B0-DET-001
#[test]
fn security_only_dimension_adds_its_two_hop_join_path() {
    let result = run(
        RETAIL,
        QUERY,
        r#"[{"member":"regions.region_name","operator":"equals","values":["North"]}]"#,
    )
    .unwrap();
    assert_eq!(result.parameters, vec![Parameter::Text("North".to_owned())]);
    insta::assert_snapshot!(format!(
        "{}\n-- parameters: {:?}",
        result.sql, result.parameters
    ));
}

// scenario: B0-SEC-003
#[test]
fn forbidden_security_only_join_trees_fail_without_sql() {
    let third_cube = RETAIL.replace(
        "{CUBE}.store_cd = {stores.store_cd}",
        "{CUBE}.store_cd = {stores.store_cd} AND {regions.region_cd} = 'R1'",
    );
    for (model, query, member, message) in [
        (RETAIL, QUERY, "receipts.printed", ""),
        (
            RETAIL,
            r#"{"measures":["receipts.tax"]}"#,
            "regions.region_name",
            "connect",
        ),
        (
            RETAIL,
            r#"{"measures":["refunds.amount"]}"#,
            "regions.region_name",
            "path",
        ),
        (RETAIL, QUERY, "visits.visit_id", "data source"),
        (
            third_cube.as_str(),
            QUERY,
            "stores.store_name",
            "cannot be ordered",
        ),
    ] {
        if message.is_empty() {
            assert!(run(
                model,
                query,
                &format!(r#"[{{"member":"{member}","operator":"set"}}]"#)
            )
            .is_ok());
            continue;
        }
        let result = run(
            model,
            query,
            &format!(r#"[{{"member":"{member}","operator":"set"}}]"#),
        );
        assert!(
            matches!(&result, Err(CompileError::Query(text)) if text.contains(message)),
            "{result:?}"
        );
    }
}
