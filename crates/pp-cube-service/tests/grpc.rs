use pp_cube_service::{
    grpc::{
        wire::{cube_engine_client::CubeEngineClient, cube_engine_server::CubeEngineServer, *},
        GrpcService,
    },
    CubeService,
};
use std::sync::Arc;

// scenario: B0-API-004
// scenario: B0-API-010
// scenario: B0-DORIS-003
#[tokio::test(flavor = "current_thread")]
async fn real_tcp_operations_preserve_compilation_validation_and_public_metadata() {
    let mut core = CubeService::new([
        ("warehouse".to_owned(), pp_cube_compile::DialectId::Presto),
        ("lake".to_owned(), pp_cube_compile::DialectId::Doris),
    ]);
    let source = "cubes:\n- name: sales\n  data_source: warehouse\n  sql_table: dw.sales\n  measures:\n  - {name: amount, type: sum, sql: '{CUBE}.amount'}\n  dimensions:\n  - {name: region, type: string, sql: '{CUBE}.region'}\n  - {name: hidden, type: string, sql: '{CUBE}.hidden', public: false}\n  segments:\n  - {name: north, sql: \"{CUBE}.region = 'north'\"}\n- name: visits\n  data_source: lake\n  sql_table: web.visit\n  measures:\n  - {name: total, type: count}\n";
    core.register("v1", "digest", pp_cube_model::load_yaml(source).unwrap())
        .unwrap();
    let core = Arc::new(core);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (stop, stopping) = tokio::sync::oneshot::channel();
    let server = tonic::transport::Server::builder()
        .add_service(CubeEngineServer::new(
            GrpcService::new(Arc::clone(&core), 2, 4).unwrap(),
        ))
        .serve_with_incoming_shutdown(
            tokio_stream::wrappers::TcpListenerStream::new(listener),
            async {
                let _ = stopping.await;
            },
        );
    let server = tokio::spawn(server);
    let mut client = CubeEngineClient::connect(format!("http://{address}"))
        .await
        .unwrap();
    let request = CompileRequest {
        model_version: "v1".into(),
        query_json: r#"{"measures":["sales.amount"]}"#.into(),
        dialect: "presto".into(),
        timezone: "Asia/Tokyo".into(),
        security_filters_json: vec![
            r#"{"member":"sales.region","operator":"equals","values":["north"]}"#.into(),
        ],
        request_id: "network-test".into(),
    };
    let response = client.compile(request.clone()).await.unwrap().into_inner();
    let identity = response.identity.clone().unwrap();
    assert_eq!(
        (
            identity.model_version.as_str(),
            identity.model_digest.as_str()
        ),
        ("v1", "digest")
    );
    assert_eq!(identity.kernel_version, pp_cube_compile::KERNEL_VERSION);
    assert_eq!(identity.upstream_pin, pp_cube_compile::UPSTREAM_PIN);
    let Some(compile_response::Result::Compiled(result)) = response.result else {
        panic!("expected compiled result")
    };
    let context = pp_cube_compile::CompileContext {
        request_id: request.request_id.clone(),
        dialect: pp_cube_compile::DialectId::Presto,
        timezone: request.timezone.clone(),
        security_filters: pp_cube_model::load_query(
            r#"{"filters":[{"member":"sales.region","operator":"equals","values":["north"]}]}"#,
        )
        .unwrap()
        .filters,
    };
    let expected = core
        .compile(
            "v1",
            &pp_cube_model::load_query(&request.query_json).unwrap(),
            &context,
        )
        .unwrap();
    assert_eq!(result.sql, expected.sql);
    assert_eq!(result.fingerprint, expected.fingerprint);
    // Typed values in placeholder order, structured lineage, and no session need without an offset.
    assert_eq!(
        result.params,
        vec![Parameter {
            value: Some(parameter::Value::Text("north".into()))
        }]
    );
    assert_eq!(
        result.used_cubes,
        vec![UsedCube {
            name: "sales".into(),
            data_source: "warehouse".into(),
            tables: vec![Table {
                path: vec!["dw".into(), "sales".into()]
            }],
        }]
    );
    assert!(result.session_properties.is_empty());
    let mut paged = request.clone();
    paged.query_json = r#"{"measures":["sales.amount"],"limit":5,"offset":10}"#.into();
    let Some(compile_response::Result::Compiled(paged)) =
        client.compile(paged).await.unwrap().into_inner().result
    else {
        panic!("expected compiled result")
    };
    assert_eq!(
        paged.session_properties,
        vec![SessionProperty {
            name: "offset_clause_enabled".into(),
            value: "true".into()
        }]
    );
    let expected_columns: Vec<_> = expected
        .columns
        .into_iter()
        .map(|c| OutputColumn {
            alias: c.alias,
            member: c.member,
            r#type: serde_json::to_value(c.semantic_type)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned(),
            granularity: c.granularity,
        })
        .collect();
    assert_eq!(result.columns, expected_columns);
    let meta = client
        .get_meta(GetMetaRequest {
            model_version: "v1".into(),
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(meta.identity, Some(identity.clone()));
    assert_eq!(
        meta.cubes
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>(),
        ["sales", "visits"]
    );
    assert_eq!(meta.cubes[0].segments, ["north"]);
    assert_eq!(meta.cubes[0].data_source, "warehouse");
    assert_eq!(meta.cubes[0].measures[0].r#type, "sum");
    assert_eq!(meta.cubes[0].dimensions.len(), 1);
    assert_eq!(meta.cubes[0].dimensions[0].r#type, "string");
    let text = format!("{meta:?}");
    assert!(!text.contains("hidden") && !text.contains("dw.sales"));
    let validation = client
        .validate_model(ValidateModelRequest {
            model_bundle: b"cubes: [{name: invalid}]".to_vec(),
        })
        .await
        .unwrap()
        .into_inner();
    assert!(!validation.issues.is_empty());
    assert!(validation.issues.iter().any(|i| !i.path.is_empty()));
    assert!(client
        .validate_model(ValidateModelRequest {
            model_bundle: source.as_bytes().to_vec()
        })
        .await
        .unwrap()
        .into_inner()
        .issues
        .is_empty());
    // JSON surrogate pairs must use the JSON parser rather than the YAML scanner.
    let json_source = r#"{"cubes":[{"name":"sales","data_source":"warehouse","title":"\ud83d\ude00","sql_table":"dw.sales","measures":[{"name":"amount","type":"sum","sql":"{CUBE}.amount"}]}]}"#;
    assert!(pp_cube_model::validate_json(json_source).is_empty());
    assert!(client
        .validate_model(ValidateModelRequest {
            model_bundle: json_source.as_bytes().to_vec()
        })
        .await
        .unwrap()
        .into_inner()
        .issues
        .is_empty());
    let mut doris = request.clone();
    doris.dialect = "doris".into();
    doris.query_json = r#"{"measures":["visits.total"]}"#.into();
    doris.security_filters_json.clear();
    let result = client.compile(doris.clone()).await.unwrap().into_inner();
    assert!(
        matches!(result.result, Some(compile_response::Result::Compiled(value)) if value.sql.contains("`visits`") && value.used_cubes[0].data_source == "lake")
    );
    // The same query for the dialect that does not serve its data source is a query failure.
    doris.dialect = "presto".into();
    let mismatch = client.compile(doris).await.unwrap().into_inner();
    assert_eq!(mismatch.identity, Some(identity.clone()));
    assert!(
        matches!(mismatch.result, Some(compile_response::Result::Failure(CompileFailure { kind, message })) if kind == FailureKind::Query as i32 && message.contains("lake"))
    );
    let mut druid = request.clone();
    druid.dialect = "druid".into();
    assert!(
        matches!(client.compile(druid).await.unwrap().into_inner().result, Some(compile_response::Result::Failure(CompileFailure { kind, .. })) if kind == FailureKind::Unsupported as i32)
    );
    let mut unknown = request.clone();
    unknown.model_version = "missing".into();
    assert_eq!(
        client.compile(unknown).await.unwrap_err().code(),
        tonic::Code::NotFound
    );
    let mut bad = request.clone();
    bad.security_filters_json = vec!["{}".into()];
    assert_eq!(
        client.compile(bad).await.unwrap_err().code(),
        tonic::Code::InvalidArgument
    );
    let mut malformed = request.clone();
    malformed.query_json = "{".into();
    assert_eq!(
        client.compile(malformed).await.unwrap_err().code(),
        tonic::Code::InvalidArgument
    );
    let mut invalid = request;
    invalid.query_json = r#"{"measures":["sales.missing"]}"#.into();
    assert!(
        matches!(client.compile(invalid).await.unwrap().into_inner().result, Some(compile_response::Result::Failure(CompileFailure { kind, .. })) if kind == FailureKind::Query as i32)
    );
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
}
