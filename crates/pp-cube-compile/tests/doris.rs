mod support;
use pp_cube_compile::{compile, CompileContext, CompileError, DialectId};
use std::sync::Arc;
fn model() -> Arc<pp_cube_model::Model> {
    Arc::new(pp_cube_model::load_yaml("cubes:\n- name: sales\n  data_source: warehouse\n  sql_table: dw.sales\n  measures:\n  - {name: count, type: count}\n  - {name: total, type: sum, sql: '{CUBE}.amount'}\n  - {name: average, type: avg, sql: '{CUBE}.amount'}\n  - {name: minimum, type: min, sql: '{CUBE}.amount'}\n  - {name: maximum, type: max, sql: '{CUBE}.amount'}\n  - {name: unique, type: count_distinct, sql: '{CUBE}.amount'}\n  - {name: restricted, type: sum, sql: '{CUBE}.amount', filters: [{sql: \"{CUBE}.region = 'north'\"}]}\n  - {name: ratio, type: number, sql: '{sales.total} / NULLIF({sales.count}, 0)'}\n  segments:\n  - {name: positive, sql: '{CUBE}.amount > 0'}\n  dimensions:\n  - {name: region, type: string, sql: '{CUBE}.region'}\n").unwrap())
}
fn context() -> CompileContext {
    CompileContext {
        request_id: "doris".into(),
        dialect: DialectId::Doris,
        timezone: "UTC".into(),
        security_filters: pp_cube_model::load_query(
            r#"{"filters":[{"member":"sales.region","operator":"equals","values":["north"]}]}"#,
        )
        .unwrap()
        .filters,
    }
}
// scenario: B0-DORIS-003
#[test]
fn basic_doris_sql_and_identity() {
    let query = pp_cube_model::load_query(r#"{"measures":["sales.count","sales.total","sales.average","sales.minimum","sales.maximum","sales.unique"],"dimensions":["sales.region"],"filters":[{"member":"sales.total","operator":"gt","values":["10"]}],"order":[["sales.total","desc"]],"limit":10,"offset":2}"#).unwrap();
    let result = support::compile(&model(), &query, &context()).unwrap();
    insta::assert_snapshot!(result.sql);
    assert_eq!(
        result.parameters,
        vec![
            pp_cube_compile::Parameter::Text("north".into()),
            pp_cube_compile::Parameter::Text("10".into())
        ]
    );
    let mut presto = context();
    presto.dialect = DialectId::Presto;
    let other = compile(&model(), &query, &presto).unwrap();
    assert_eq!(result.columns, other.columns);
    assert_eq!(result.used_cubes, other.used_cubes);
    assert_ne!(result.fingerprint, other.fingerprint);
}
// scenario: B0-DORIS-004
#[test]
fn unsupported_doris_features_fail_closed() {
    let query = pp_cube_model::load_query(r#"{"measures":["sales.total"],"offset":2}"#).unwrap();
    assert!(matches!(
        compile(&model(), &query, &context()),
        Err(CompileError::Unsupported(_))
    ));
    let mut invalid = (*model()).clone();
    invalid.cubes[0].sql_table = None;
    assert!(matches!(
        compile(&Arc::new(invalid), &query, &context()),
        Err(CompileError::Model(_))
    ));
}

// scenario: B0-DORIS-003
#[test]
fn formulas_segments_and_presence_filters_compile_without_limit() {
    let query = pp_cube_model::load_query(r#"{"measures":["sales.restricted","sales.ratio"],"segments":["sales.positive"],"filters":[{"or":[{"member":"sales.region","operator":"set"},{"member":"sales.region","operator":"notSet"}]}],"order":[]}"#).unwrap();
    let result = support::compile(&model(), &query, &context()).unwrap();
    insta::assert_snapshot!(result.sql);
    assert_eq!(
        result.parameters,
        vec![pp_cube_compile::Parameter::Text("north".into())]
    );
}

fn retail() -> Arc<pp_cube_model::Model> {
    Arc::new(pp_cube_model::load_yaml(include_str!("fixtures/retail.yaml")).unwrap())
}
fn join_context() -> CompileContext {
    let mut ctx = context();
    ctx.security_filters = pp_cube_model::load_query(
        r#"{"filters":[{"member":"regions.region_name","operator":"equals","values":["north"]}]}"#,
    )
    .unwrap()
    .filters;
    ctx
}
// scenario: B0-DORIS-005
#[test]
fn joined_dimensions_and_security_only_paths() {
    for (name, text) in [
        (
            "grouped",
            r#"{"measures":["sales.amount"],"dimensions":["stores.store_name"],"filters":[{"member":"stores.store_name","operator":"equals","values":["central"]}]}"#,
        ),
        ("security_only", r#"{"measures":["sales.amount"]}"#),
    ] {
        let query = pp_cube_model::load_query(text).unwrap();
        let result = support::compile(&retail(), &query, &join_context()).unwrap();
        insta::assert_snapshot!(
            format!("doris_join_{name}"),
            format!("{}\n-- parameters: {:?}", result.sql, result.parameters)
        );
        let mut ctx = join_context();
        ctx.dialect = DialectId::Presto;
        let expected = compile(&retail(), &query, &ctx).unwrap();
        assert_eq!(result.parameters, expected.parameters);
        assert_eq!(result.columns, expected.columns);
        assert_eq!(result.used_cubes, expected.used_cubes);
        let tables: Vec<_> = result
            .used_cubes
            .iter()
            .flat_map(|c| c.physical_tables.iter().map(|p| p.join(".")))
            .collect();
        for table in ["dw.sales_dtl", "dw.dim_store", "dw.dim_region"] {
            assert!(tables.iter().any(|t| t == table));
        }
    }
}
// scenario: B0-DORIS-006
#[test]
fn doris_retains_join_boundaries() {
    for text in [
        r#"{"dimensions":["sales.channel_cd","refunds.refund_id","stores.store_name"]}"#,
        r#"{"measures":["refunds.amount"],"dimensions":["regions.region_name"]}"#,
        r#"{"measures":["sales.amount"],"dimensions":["visits.source"]}"#,
    ] {
        assert!(matches!(
            compile(
                &retail(),
                &pp_cube_model::load_query(text).unwrap(),
                &join_context()
            ),
            Err(CompileError::Query(_))
        ));
    }
    let query = pp_cube_model::load_query(
        r#"{"measures":["sales.amount","stores.store_count"],"dimensions":["stores.store_name"]}"#,
    )
    .unwrap();
    assert!(matches!(
        compile(&retail(), &query, &join_context()),
        Err(CompileError::Unsupported(_))
    ));
    // A sum over a cube whose rows a join repeats needs multi-stage SQL on Doris too.
    let query = pp_cube_model::load_query(
        r#"{"measures":["stores.floor_area"],"dimensions":["sales.channel_cd"]}"#,
    )
    .unwrap();
    assert!(matches!(
        compile(&retail(), &query, &join_context()),
        Err(CompileError::Unsupported(message)) if message.contains("stores.floor_area")
    ));
}

// scenario: B0-DORIS-007
#[test]
fn doris_literal_patterns() {
    let mut snapshots = Vec::new();
    for operator in [
        "contains",
        "notContains",
        "startsWith",
        "notStartsWith",
        "endsWith",
        "notEndsWith",
    ] {
        let text = format!(
            r#"{{"measures":["sales.total"],"filters":[{{"member":"sales.region","operator":"{operator}","values":["A%_\\B"]}}]}}"#
        );
        let query = pp_cube_model::load_query(&text).unwrap();
        let result = support::compile(&model(), &query, &context()).unwrap();
        let mut presto = context();
        presto.dialect = DialectId::Presto;
        let expected = compile(&model(), &query, &presto).unwrap();
        assert_eq!(result.parameters, expected.parameters);
        assert_eq!(result.columns, expected.columns);
        assert_eq!(result.used_cubes, expected.used_cubes);
        snapshots.push(format!(
            "{operator}:\n{}\n{:?}",
            result.sql, result.parameters
        ));
    }
    insta::assert_snapshot!(snapshots.join("\n\n"));
}
// scenario: B0-DORIS-008
#[test]
fn doris_security_pattern_conjoins_user_or() {
    let mut ctx = context();
    ctx.security_filters = pp_cube_model::load_query(
        r#"{"filters":[{"member":"sales.region","operator":"startsWith","values":["north_"]}]}"#,
    )
    .unwrap()
    .filters;
    let query = pp_cube_model::load_query(r#"{"measures":["sales.total"],"filters":[{"or":[{"member":"sales.region","operator":"contains","values":["central%"]},{"member":"sales.region","operator":"endsWith","values":["west"]}]}]}"#).unwrap();
    let result = support::compile(&model(), &query, &ctx).unwrap();
    insta::assert_snapshot!(format!("{}\n{:?}", result.sql, result.parameters));
}
