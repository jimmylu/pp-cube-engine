use pp_cube_compile::{compile, CompileContext, DialectId};
use pp_cube_model::{load_yaml, Query};
use pp_cube_service::{CubeService, ServiceError};

fn model() -> pp_cube_model::Model {
    load_yaml("cubes:\n- name: sales\n  sql_table: dw.sales\n  data_source: warehouse\n  measures:\n  - {name: amount, type: sum, sql: '{CUBE}.amount'}\n  dimensions:\n  - {name: region, type: string, sql: '{CUBE}.region'}\n  - {name: hidden, type: string, sql: '{CUBE}.secret', public: false}\n").unwrap()
}
fn context() -> CompileContext {
    CompileContext {
        request_id: "service-test".into(),
        dialect: DialectId::Presto,
        timezone: "Asia/Tokyo".into(),
        security_filters: vec![],
    }
}
// scenario: B0-API-001
#[test]
fn versioned_operations_return_facade_result_and_only_public_metadata() {
    let mut service = CubeService::default();
    let mut m = model();
    m.cubes[0].title = Some("Sales".into());
    m.cubes[0].description = Some("Public catalog".into());
    m.cubes[0].dimensions[0].title = Some("Region".into());
    m.cubes[0].dimensions[0].public = Some(true);
    let hidden = m.cubes[0].measures[0].clone();
    m.cubes[0].measures.push(hidden);
    m.cubes[0].measures[1].name = "hidden_amount".into();
    m.cubes[0].measures[1].public = Some(false);
    let mut hidden_cube = m.cubes[0].clone();
    hidden_cube.name = "hidden_cube".into();
    hidden_cube.public = Some(false);
    m.cubes.push(hidden_cube);
    service.register("v1", "digest", m.clone()).unwrap();
    let query = Query {
        measures: vec!["sales.amount".into()],
        ..Query::default()
    };
    assert_eq!(
        service.compile("v1", &query, &context()).unwrap(),
        compile(&std::sync::Arc::new(m), &query, &context()).unwrap()
    );
    assert!(matches!(
        service.compile("missing", &query, &context()),
        Err(ServiceError::Bundle(_))
    ));
    assert!(service.get_meta("missing").is_err());
    let mut invalid = model();
    invalid.cubes[0].measures[0].sql = None;
    let issues = service.validate_model(&invalid);
    assert!(!issues.is_empty());
    assert!(issues.iter().all(|issue| !issue.path.is_empty()));
    assert_eq!(issues, pp_cube_model::validate_model(&invalid));
    let meta = service.get_meta("v1").unwrap();
    assert_eq!(meta.len(), 1);
    assert_eq!(meta[0].name, "sales");
    assert_eq!(meta[0].title.as_deref(), Some("Sales"));
    assert_eq!(meta[0].description.as_deref(), Some("Public catalog"));
    assert_eq!(
        meta[0].measures[0].member_type,
        pp_cube_model::MeasureType::Sum
    );
    assert_eq!(
        meta[0].dimensions[0].member_type,
        pp_cube_model::DimensionType::String
    );
    assert_eq!(meta[0].dimensions[0].title.as_deref(), Some("Region"));
    assert_eq!(meta[0].measures.len(), 1);
    assert_eq!(meta[0].measures[0].name, "amount");
    assert_eq!(meta[0].dimensions.len(), 1);
    assert_eq!(meta[0].dimensions[0].name, "region");
}

// scenario: B0-API-002
#[test]
fn invalid_registration_and_validation_preserve_available_versions() {
    use pp_cube_model::BundleError;
    let mut service = CubeService::default();
    service.register("v1", "digest", model()).unwrap();
    let mut invalid = model();
    invalid.cubes[0].measures[0].sql = None;
    let issues = service.validate_model(&invalid);
    assert!(!issues.is_empty());
    assert!(issues.iter().all(|issue| !issue.path.is_empty()));
    assert_eq!(
        service.register("v2", "digest", invalid).unwrap_err(),
        BundleError::Model(issues)
    );
    assert_eq!(service.get_meta("v2"), Err(BundleError::UnknownVersion));
    assert!(service.validate_model(&model()).is_empty());
    assert_eq!(service.get_meta("v1").unwrap()[0].name, "sales");
    let query = Query {
        measures: vec!["sales.amount".into()],
        ..Query::default()
    };
    assert!(service.compile("v1", &query, &context()).is_ok());
    assert_eq!(
        service.register("v1", "different", model()).unwrap_err(),
        BundleError::DuplicateVersion
    );
}

// scenario: B0-API-003
#[test]
fn complete_context_and_security_reach_the_facade_unchanged() {
    use pp_cube_compile::CompileError;
    use pp_cube_model::{load_query, Filter};
    let mut service = CubeService::default();
    let first = service.register("v1", "digest", model()).unwrap();
    let mut changed = model();
    changed.cubes[0].sql_table = Some("dw.sales_v2".into());
    service.register("v2", "digest2", changed).unwrap();
    let query = load_query(r#"{"measures":["sales.amount"],"filters":[{"or":[{"member":"sales.region","operator":"equals","values":["east"]},{"member":"sales.region","operator":"equals","values":["west"]}]}]}"#).unwrap();
    let mut ctx = context();
    ctx.security_filters = load_query(
        r#"{"filters":[{"member":"sales.region","operator":"equals","values":["north"]}]}"#,
    )
    .unwrap()
    .filters;
    let actual = service.compile("v1", &query, &ctx).unwrap();
    assert_eq!(actual, compile(first.model(), &query, &ctx).unwrap());
    assert!(actual.sql.contains(" OR ") && actual.sql.contains(" AND "));
    assert_eq!(actual.parameters.len(), 3);
    assert_ne!(actual, service.compile("v1", &query, &context()).unwrap());
    assert!(service
        .compile("v2", &query, &ctx)
        .unwrap()
        .sql
        .contains("dw.sales_v2"));
    let mut correlation = ctx.clone();
    correlation.request_id = "other".into();
    assert_eq!(actual, service.compile("v1", &query, &correlation).unwrap());
    ctx.security_filters = vec![Filter::Or(vec![])];
    assert!(matches!(
        service.compile("v1", &query, &ctx),
        Err(ServiceError::Compile(CompileError::Query(_)))
    ));
}
