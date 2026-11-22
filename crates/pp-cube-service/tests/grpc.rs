use pp_cube_service::{
    grpc::{
        wire::{cube_engine_client::CubeEngineClient, cube_engine_server::CubeEngineServer, *},
        GrpcService,
    },
    CubeService,
};
use std::sync::Arc;

// scenario: B0-API-004
// scenario: B0-DORIS-003
#[tokio::test(flavor = "current_thread")]
async fn real_tcp_operations_preserve_compilation_validation_and_public_metadata() {
    let mut core = CubeService::default();
    let source = "cubes:\n- name: sales\n  data_source: warehouse\n  sql_table: dw.sales\n  measures:\n  - {name: amount, type: sum, sql: '{CUBE}.amount'}\n  dimensions:\n  - {name: region, type: string, sql: '{CUBE}.region'}\n  - {name: hidden, type: string, sql: '{CUBE}.hidden', public: false}\n";
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
    let result = client.compile(request.clone()).await.unwrap().into_inner();
    let Some(compile_response::Result::Compiled(result)) = result.result else {
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
    assert_eq!(result.params, vec!["\"north\""]);
    assert_eq!(result.used_tables, vec!["dw.sales"]);
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
    assert!(!meta.meta_json.contains("hidden"));
    assert!(!meta.meta_json.contains("dw.sales"));
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
    let result = client.compile(doris).await.unwrap().into_inner();
    assert!(
        matches!(result.result, Some(compile_response::Result::Compiled(value)) if value.sql.contains("`sales`"))
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
        matches!(client.compile(invalid).await.unwrap().into_inner().result, Some(compile_response::Result::Failure(CompileFailure { kind, .. })) if kind == "query")
    );
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
}
