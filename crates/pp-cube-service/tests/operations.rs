use pp_cube_compile::{compile, CompileContext, DialectId};
use pp_cube_model::{load_yaml, Query};
use pp_cube_service::{CubeService, ServiceError};

fn model() -> pp_cube_model::Model {
    load_yaml("cubes:\n- name: sales\n  sql_table: dw.sales\n  data_source: warehouse\n  measures:\n  - {name: amount, type: sum, sql: '{CUBE}.amount'}\n  dimensions:\n  - {name: region, type: string, sql: '{CUBE}.region'}\n  - {name: hidden, type: string, sql: '{CUBE}.secret', public: false}\n").unwrap()
}
fn service() -> CubeService {
    CubeService::new([
        ("warehouse".to_owned(), DialectId::Presto),
        ("lake".to_owned(), DialectId::Doris),
    ])
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
    let mut service = service();
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
    let mut service = service();
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
    let mut service = service();
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

// scenario: B0-API-011
#[test]
fn the_requested_dialect_must_serve_every_data_source_the_query_reads() {
    use pp_cube_compile::CompileError;
    use pp_cube_model::BundleError;
    let source = "cubes:\n- name: sales\n  sql_table: dw.sales\n  data_source: warehouse\n  measures:\n  - {name: amount, type: sum, sql: '{CUBE}.amount'}\n- name: visits\n  sql_table: web.visit\n  data_source: lake\n  measures:\n  - {name: total, type: count}\n";
    let mut service = service();
    service
        .register("v1", "digest", load_yaml(source).unwrap())
        .unwrap();
    let query = |member: &str| Query {
        measures: vec![member.into()],
        ..Query::default()
    };
    let doris = CompileContext {
        dialect: DialectId::Doris,
        ..context()
    };
    // Each data source compiles for the dialect that serves it and for no other.
    assert!(service
        .compile("v1", &query("sales.amount"), &context())
        .unwrap()
        .sql
        .contains('"'));
    assert!(service
        .compile("v1", &query("visits.total"), &doris)
        .unwrap()
        .sql
        .contains('`'));
    for (member, ctx, cube, source, dialect) in [
        ("sales.amount", &doris, "sales", "warehouse", "doris"),
        ("visits.total", &context(), "visits", "lake", "presto"),
    ] {
        let Err(ServiceError::Compile(CompileError::Query(message))) =
            service.compile("v1", &query(member), ctx)
        else {
            panic!("{member} must not compile for {dialect}")
        };
        for part in [cube, source, dialect] {
            assert!(message.contains(part), "{message}");
        }
        assert!(
            !message.contains("dw.") && !message.contains("web."),
            "{message}"
        );
    }
    // A model on a data source without a configured dialect never becomes available.
    let unmapped = source.replace("data_source: lake", "data_source: archive");
    assert_eq!(
        service
            .register("v2", "digest", load_yaml(&unmapped).unwrap())
            .unwrap_err(),
        BundleError::UnmappedDataSource {
            data_source: "archive".into()
        }
    );
    assert_eq!(service.get_meta("v2"), Err(BundleError::UnknownVersion));
    assert!(service
        .compile("v1", &query("sales.amount"), &context())
        .is_ok());
}

// scenario: B0-API-012
#[test]
fn metadata_describes_public_members_without_sql_or_tables() {
    let source = r#"cubes:
- name: sales
  sql_table: dw.sales_dtl
  data_source: warehouse
  meta: {owner: retail, tags: [daily]}
  joins:
  - {name: stores, relationship: many_to_one, sql: "{CUBE}.store_cd = {stores.store_cd}"}
  - {name: audits, relationship: one_to_one, sql: "{CUBE}.sale_id = {audits.sale_id}"}
  dimensions:
  - {name: sale_id, type: number, sql: "{CUBE}.sale_id", primary_key: true}
  - {name: region, type: string, sql: "{CUBE}.region_cd", meta: {unit: code}}
  measures:
  - {name: amount, type: sum, sql: "{CUBE}.amount", format: currency, meta: {precision: 2}}
  - {name: cost, type: sum, sql: "{CUBE}.secret_cost", public: false, meta: {internal: true}}
  segments:
  - {name: toys, sql: "{CUBE}.category_cd = 'TOY'"}
- name: stores
  sql_table: dw.dim_store
  data_source: warehouse
  dimensions:
  - {name: store_cd, type: string, sql: "{CUBE}.store_cd", primary_key: true}
- name: audits
  sql_table: dw.audit_log
  data_source: warehouse
  public: false
  dimensions:
  - {name: sale_id, type: number, sql: "{CUBE}.sale_id", primary_key: true}
"#;
    let mut service = service();
    service
        .register("v1", "digest", load_yaml(source).unwrap())
        .unwrap();
    let meta = service.get_meta("v1").unwrap();
    assert_eq!(
        meta.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
        ["sales", "stores"]
    );
    let sales = &meta[0];
    assert_eq!(sales.data_source, "warehouse");
    assert_eq!(
        sales.meta,
        Some(serde_json::json!({"owner": "retail", "tags": ["daily"]}))
    );
    assert_eq!(sales.segments, ["toys"]);
    // The join to the hidden cube is not discoverable.
    assert_eq!(sales.joins.len(), 1);
    assert_eq!(sales.joins[0].cube, "stores");
    assert_eq!(
        sales.joins[0].relationship,
        pp_cube_model::Relationship::ManyToOne
    );
    assert_eq!(sales.measures.len(), 1);
    assert_eq!(sales.measures[0].format.as_deref(), Some("currency"));
    assert_eq!(
        sales.measures[0].meta,
        Some(serde_json::json!({"precision": 2}))
    );
    assert!(sales.dimensions[0].primary_key && !sales.dimensions[1].primary_key);
    assert_eq!(
        sales.dimensions[1].meta,
        Some(serde_json::json!({"unit": "code"}))
    );
    let text = serde_json::to_string(&meta).unwrap();
    for hidden in [
        "dw.",
        "secret_cost",
        "region_cd",
        "category_cd",
        "audits",
        "internal",
        "store_cd =",
    ] {
        assert!(!text.contains(hidden), "{hidden} leaked: {text}");
    }
    let identity = service.identity("v1").unwrap();
    assert_eq!(identity.model_version, "v1");
    assert_eq!(identity.model_digest, "digest");
    assert_eq!(identity.kernel_version, pp_cube_compile::KERNEL_VERSION);
    assert_eq!(identity.upstream_pin.len(), 40);
    assert!(service.identity("missing").is_err());
}
