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
    let service = service();
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
    service.publish("v1", m.clone(), None).unwrap();
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
    let service = service();
    service.publish("v1", model(), None).unwrap();
    let mut invalid = model();
    invalid.cubes[0].measures[0].sql = None;
    let issues = service.validate_model(&invalid);
    assert!(!issues.is_empty());
    assert!(issues.iter().all(|issue| !issue.path.is_empty()));
    assert_eq!(
        service.publish("v2", invalid, None).unwrap_err(),
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
    let mut changed = model();
    changed.cubes[0].sql_table = Some("dw.other".into());
    assert_eq!(
        service.publish("v1", changed, None).unwrap_err(),
        BundleError::DuplicateVersion
    );
}

// scenario: B0-API-003
#[test]
fn complete_context_and_security_reach_the_facade_unchanged() {
    use pp_cube_compile::CompileError;
    use pp_cube_model::{load_query, Filter};
    let service = service();
    let first = service.publish("v1", model(), None).unwrap().bundle;
    let mut changed = model();
    changed.cubes[0].sql_table = Some("dw.sales_v2".into());
    service.publish("v2", changed, None).unwrap();
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
    let service = service();
    service
        .publish("v1", load_yaml(source).unwrap(), None)
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
            .publish("v2", load_yaml(&unmapped).unwrap(), None)
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
    let service = service();
    service
        .publish("v1", load_yaml(source).unwrap(), None)
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
    assert_eq!(
        identity.model_digest,
        pp_cube_compile::model_digest(&load_yaml(source).unwrap()).unwrap()
    );
    assert_eq!(identity.kernel_version, pp_cube_compile::KERNEL_VERSION);
    assert_eq!(identity.upstream_pin.len(), 40);
    assert!(service.identity("missing").is_err());
}

// scenario: B0-API-013
#[test]
fn versions_are_published_listed_and_retired_without_disturbing_others() {
    use pp_cube_model::BundleError;
    let service = std::sync::Arc::new(service());
    let query = Query {
        measures: vec!["sales.amount".into()],
        ..Query::default()
    };
    let mut second = model();
    second.cubes[0].sql_table = Some("dw.sales_v2".into());
    assert!(service.versions().is_empty());
    assert!(service.publish("v2", second.clone(), None).unwrap().created);
    assert!(service.publish("v1", model(), None).unwrap().created);
    // Ordered by version, each with its content digest.
    let versions = service.versions();
    assert_eq!(
        versions.iter().map(|v| v.0.as_str()).collect::<Vec<_>>(),
        ["v1", "v2"]
    );
    assert_ne!(versions[0].1, versions[1].1);
    // The same content again is accepted and changes nothing; other content is refused.
    let again = service.publish("v1", model(), None).unwrap();
    assert!(!again.created);
    assert_eq!(again.bundle.digest(), versions[0].1);
    assert_eq!(
        service.publish("v1", second, None).unwrap_err(),
        BundleError::DuplicateVersion
    );
    assert!(service
        .compile("v1", &query, &context())
        .unwrap()
        .sql
        .contains("dw.sales "));
    // Publishing and retiring while other threads compile never breaks a selected version.
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let (service, query) = (std::sync::Arc::clone(&service), query.clone());
            std::thread::spawn(move || {
                for _ in 0..50 {
                    assert!(service
                        .compile("v2", &query, &context())
                        .unwrap()
                        .sql
                        .contains("dw.sales_v2"));
                }
            })
        })
        .collect();
    for round in 0..20 {
        let version = format!("temp{round}");
        service.publish(version.clone(), model(), None).unwrap();
        service.retire(&version).unwrap();
    }
    for worker in workers {
        worker.join().unwrap();
    }
    // A retired version is gone for new requests; work that already holds it is unaffected.
    let held = service.retire("v1").unwrap();
    assert_eq!(held.version(), "v1");
    assert!(pp_cube_compile::compile(held.model(), &query, &context()).is_ok());
    assert!(matches!(
        service.compile("v1", &query, &context()),
        Err(ServiceError::Bundle(BundleError::UnknownVersion))
    ));
    assert_eq!(service.get_meta("v1"), Err(BundleError::UnknownVersion));
    assert_eq!(
        service.retire("v1").unwrap_err(),
        BundleError::UnknownVersion
    );
    assert_eq!(service.versions().len(), 1);
    assert!(service.compile("v2", &query, &context()).is_ok());
    // A retired version name can be published again, with any content.
    assert!(service.publish("v1", model(), None).unwrap().created);
}

// scenario: B0-API-014
#[test]
fn the_digest_is_of_the_content_and_an_expected_digest_must_match() {
    use pp_cube_model::BundleError;
    let yaml = "# a comment\ncubes:\n- name: sales\n  data_source: warehouse\n  sql_table: dw.sales\n  measures:\n  - {name: amount, type: sum, sql: '{CUBE}.amount'}\n";
    let json = r#"{"cubes":[{"measures":[{"sql":"{CUBE}.amount","type":"sum","name":"amount"}],"sql_table":"dw.sales","name":"sales","data_source":"warehouse"}]}"#;
    let digest = pp_cube_compile::model_digest(&load_yaml(yaml).unwrap()).unwrap();
    // Formatting, key order and comments do not matter; content does.
    assert_eq!(
        digest,
        pp_cube_compile::model_digest(&pp_cube_model::load_json(json).unwrap()).unwrap()
    );
    assert!(digest.starts_with("sha256:") && digest.len() == 71);
    assert!(digest[7..]
        .bytes()
        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()));
    let other = yaml.replace("dw.sales", "dw.sales_v2");
    assert_ne!(
        digest,
        pp_cube_compile::model_digest(&load_yaml(&other).unwrap()).unwrap()
    );
    let service = service();
    // A wrong expectation publishes nothing; the right one does, and identifies the version.
    assert_eq!(
        service
            .publish("v1", load_yaml(yaml).unwrap(), Some("sha256:0000"))
            .unwrap_err(),
        BundleError::DigestMismatch
    );
    assert!(service.versions().is_empty());
    let published = service
        .publish("v1", load_yaml(yaml).unwrap(), Some(&digest))
        .unwrap();
    assert!(published.created);
    assert_eq!(service.identity("v1").unwrap().model_digest, digest);
    // The JSON form is the same content, so it is the same version.
    assert!(
        !service
            .publish("v1", pp_cube_model::load_json(json).unwrap(), Some(&digest))
            .unwrap()
            .created
    );
    assert_eq!(
        service
            .publish(" ", load_yaml(yaml).unwrap(), None)
            .unwrap_err(),
        BundleError::InvalidIdentifier { field: "version" }
    );
}

// scenario: B1-API-001
#[test]
fn views_are_compiled_and_listed_with_the_members_they_take() {
    let source = r#"
cubes:
- name: sales
  sql_table: dw.sales
  data_source: warehouse
  joins:
  - {name: stores, relationship: many_to_one, sql: "{CUBE}.store_cd = {stores.store_cd}"}
  dimensions:
  - {name: sale_id, type: number, sql: "{CUBE}.sale_id", primary_key: true}
  measures:
  - {name: amount, type: sum, sql: "{CUBE}.amount", title: Amount}
  - {name: secret, type: sum, sql: "{CUBE}.cost", public: false}
- name: stores
  sql_table: dw.stores
  data_source: warehouse
  dimensions:
  - {name: store_cd, type: string, sql: "{CUBE}.store_cd", primary_key: true}
  - {name: store_name, type: string, sql: "{CUBE}.store_name"}
views:
- name: sales_view
  title: Sales by store
  cubes:
  - {join_path: sales, includes: [amount, secret]}
  - {join_path: sales.stores, includes: [store_name]}
"#;
    let service = service();
    let model = load_yaml(source).unwrap();
    service.publish("v1", model.clone(), None).unwrap();
    let query = Query {
        measures: vec!["sales_view.amount".into()],
        dimensions: vec!["sales_view.store_name".into()],
        ..Query::default()
    };
    let compiled = service.compile("v1", &query, &context()).unwrap();
    assert_eq!(
        compiled,
        compile(&std::sync::Arc::new(model), &query, &context()).unwrap()
    );
    assert!(compiled.sql.contains(r#""sales_view__amount""#));
    // Listed after the cubes, with the type of the measure behind each member
    // and the visibility the member has in its cube.
    let meta = service.get_meta("v1").unwrap();
    let names: Vec<_> = meta.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["sales", "stores", "sales_view"]);
    let view = &meta[2];
    assert_eq!(view.title.as_deref(), Some("Sales by store"));
    assert_eq!(view.data_source, "warehouse");
    assert_eq!(view.measures.len(), 1);
    assert_eq!(
        (
            view.measures[0].name.as_str(),
            view.measures[0].member_type,
            view.measures[0].title.as_deref()
        ),
        ("amount", pp_cube_model::MeasureType::Sum, Some("Amount"))
    );
    assert_eq!(view.dimensions[0].name, "store_name");
    assert!(view.joins.is_empty() && view.segments.is_empty());
}
