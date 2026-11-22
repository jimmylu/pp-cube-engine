mod support;
use pp_cube_compile::{compile, CompileContext, CompileError, DialectId};
use std::sync::Arc;
fn model() -> Arc<pp_cube_model::Model> {
    Arc::new(pp_cube_model::load_yaml("cubes:\n- name: sales\n  data_source: warehouse\n  sql_table: dw.sales\n  measures:\n  - {name: count, type: count}\n  - {name: total, type: sum, sql: '{CUBE}.amount'}\n  - {name: average, type: avg, sql: '{CUBE}.amount'}\n  - {name: minimum, type: min, sql: '{CUBE}.amount'}\n  - {name: maximum, type: max, sql: '{CUBE}.amount'}\n  - {name: unique, type: count_distinct, sql: '{CUBE}.amount'}\n  dimensions:\n  - {name: region, type: string, sql: '{CUBE}.region'}\n").unwrap())
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
    for text in [
        r#"{"measures":["sales.total"],"offset":2}"#,
        r#"{"measures":["sales.total"],"timeDimensions":[{"dimension":"sales.region","granularity":"day"}]}"#,
        r#"{"measures":["sales.total"],"filters":[{"member":"sales.region","operator":"contains","values":["n"]}]}"#,
    ] {
        assert!(matches!(
            compile(
                &model(),
                &pp_cube_model::load_query(text).unwrap(),
                &context()
            ),
            Err(CompileError::Unsupported(_))
        ));
    }
    let query = pp_cube_model::load_query(r#"{"measures":["sales.total"]}"#).unwrap();
    let mut time_model = (*model()).clone();
    time_model.cubes[0].dimensions[0].dimension_type = pp_cube_model::DimensionType::Time;
    assert!(matches!(
        compile(&Arc::new(time_model), &query, &context()),
        Err(CompileError::Unsupported(_))
    ));
    let mut multi = (*model()).clone();
    let mut second = multi.cubes[0].clone();
    second.name = "other".into();
    multi.cubes.push(second);
    assert!(matches!(
        compile(&Arc::new(multi), &query, &context()),
        Err(CompileError::Unsupported(_))
    ));
    let mut invalid = (*model()).clone();
    invalid.cubes[0].sql_table = None;
    assert!(matches!(
        compile(&Arc::new(invalid), &query, &context()),
        Err(CompileError::Model(_))
    ));
    let mut ctx = context();
    ctx.security_filters = pp_cube_model::load_query(
        r#"{"filters":[{"or":[{"member":"sales.region","operator":"contains","values":["n"]}]}]}"#,
    )
    .unwrap()
    .filters;
    assert!(matches!(
        compile(
            &model(),
            &pp_cube_model::load_query(r#"{"measures":["sales.total"]}"#).unwrap(),
            &ctx
        ),
        Err(CompileError::Unsupported(_))
    ));
}
