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
    let core = CubeService::new([
        ("warehouse".to_owned(), pp_cube_compile::DialectId::Presto),
        ("lake".to_owned(), pp_cube_compile::DialectId::Doris),
    ]);
    let source = "cubes:\n- name: sales\n  data_source: warehouse\n  sql_table: dw.sales\n  measures:\n  - {name: amount, type: sum, sql: '{CUBE}.amount'}\n  dimensions:\n  - {name: region, type: string, sql: '{CUBE}.region'}\n  - {name: hidden, type: string, sql: '{CUBE}.hidden', public: false}\n  segments:\n  - {name: north, sql: \"{CUBE}.region = 'north'\"}\n- name: visits\n  data_source: lake\n  sql_table: web.visit\n  measures:\n  - {name: total, type: count}\n";
    core.publish("v1", pp_cube_model::load_yaml(source).unwrap(), None)
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
        (
            "v1",
            pp_cube_compile::model_digest(&pp_cube_model::load_yaml(source).unwrap())
                .unwrap()
                .as_str()
        )
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

async fn serve(
    management: bool,
) -> (
    CubeEngineClient<tonic::transport::Channel>,
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<Result<(), tonic::transport::Error>>,
) {
    let core = Arc::new(CubeService::new([(
        "warehouse".to_owned(),
        pp_cube_compile::DialectId::Presto,
    )]));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (stop, stopping) = tokio::sync::oneshot::channel();
    let service = GrpcService::new(core, 2, 4)
        .unwrap()
        .with_management(management);
    let server = tokio::spawn(
        tonic::transport::Server::builder()
            .add_service(CubeEngineServer::new(service))
            .serve_with_incoming_shutdown(
                tokio_stream::wrappers::TcpListenerStream::new(listener),
                async {
                    let _ = stopping.await;
                },
            ),
    );
    let client = CubeEngineClient::connect(format!("http://{address}"))
        .await
        .unwrap();
    (client, stop, server)
}

const SALES: &str = "cubes:\n- name: sales\n  data_source: warehouse\n  sql_table: dw.sales\n  measures:\n  - {name: amount, type: sum, sql: '{CUBE}.amount'}\n";

// scenario: B0-API-015
#[tokio::test(flavor = "current_thread")]
async fn management_is_refused_unless_the_host_enabled_it() {
    let (mut client, stop, server) = serve(false).await;
    let publish = PublishModelRequest {
        model_version: "v1".into(),
        model_bundle: SALES.as_bytes().to_vec(),
        expected_digest: None,
    };
    assert_eq!(
        client.publish_model(publish).await.unwrap_err().code(),
        tonic::Code::PermissionDenied
    );
    assert_eq!(
        client
            .retire_model(RetireModelRequest {
                model_version: "v1".into()
            })
            .await
            .unwrap_err()
            .code(),
        tonic::Code::PermissionDenied
    );
    assert_eq!(
        client
            .list_models(ListModelsRequest {})
            .await
            .unwrap_err()
            .code(),
        tonic::Code::PermissionDenied
    );
    // Nothing was published, and the read operations still answer.
    assert_eq!(
        client
            .get_meta(GetMetaRequest {
                model_version: "v1".into()
            })
            .await
            .unwrap_err()
            .code(),
        tonic::Code::NotFound
    );
    let validation = client
        .validate_model(ValidateModelRequest {
            model_bundle: SALES.as_bytes().to_vec(),
        })
        .await
        .unwrap()
        .into_inner();
    assert!(validation.issues.is_empty());
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
}

// scenario: B0-API-013
// scenario: B0-API-014
#[tokio::test(flavor = "current_thread")]
async fn a_version_is_published_compiled_listed_and_retired_over_the_wire() {
    let (mut client, stop, server) = serve(true).await;
    let compile = CompileRequest {
        model_version: "v1".into(),
        query_json: r#"{"measures":["sales.amount"]}"#.into(),
        dialect: "presto".into(),
        timezone: "UTC".into(),
        ..Default::default()
    };
    assert_eq!(
        client.compile(compile.clone()).await.unwrap_err().code(),
        tonic::Code::NotFound
    );
    // ValidateModel tells a publisher the digest to expect; an invalid model has none.
    let digest = client
        .validate_model(ValidateModelRequest {
            model_bundle: SALES.as_bytes().to_vec(),
        })
        .await
        .unwrap()
        .into_inner()
        .model_digest
        .unwrap();
    let invalid = client
        .validate_model(ValidateModelRequest {
            model_bundle: b"cubes: [{name: invalid}]".to_vec(),
        })
        .await
        .unwrap()
        .into_inner();
    assert!(invalid.model_digest.is_none() && !invalid.issues.is_empty());
    let mut publish = PublishModelRequest {
        model_version: "v1".into(),
        model_bundle: SALES.as_bytes().to_vec(),
        expected_digest: Some("sha256:wrong".into()),
    };
    assert_eq!(
        client
            .publish_model(publish.clone())
            .await
            .unwrap_err()
            .code(),
        tonic::Code::FailedPrecondition
    );
    publish.expected_digest = Some(digest.clone());
    let published = |response: PublishModelResponse| match response.result {
        Some(publish_model_response::Result::Published(published)) => published,
        other => panic!("expected a published version, got {other:?}"),
    };
    let first = published(
        client
            .publish_model(publish.clone())
            .await
            .unwrap()
            .into_inner(),
    );
    assert!(first.created);
    let identity = first.identity.unwrap();
    assert_eq!(
        (identity.model_version.as_str(), &identity.model_digest),
        ("v1", &digest)
    );
    // The same content again is accepted and creates nothing; other content is refused.
    assert!(
        !published(
            client
                .publish_model(publish.clone())
                .await
                .unwrap()
                .into_inner()
        )
        .created
    );
    let mut other = publish.clone();
    other.expected_digest = None;
    other.model_bundle = SALES.replace("dw.sales", "dw.sales_v2").into_bytes();
    assert_eq!(
        client
            .publish_model(other.clone())
            .await
            .unwrap_err()
            .code(),
        tonic::Code::AlreadyExists
    );
    // An invalid model is rejected with its issues; an unmapped data source and a blank
    // version are refused; none of them becomes available.
    let mut rejected = other.clone();
    rejected.model_version = "v2".into();
    rejected.model_bundle = b"cubes: [{name: invalid}]".to_vec();
    assert!(matches!(
        client.publish_model(rejected).await.unwrap().into_inner().result,
        Some(publish_model_response::Result::Rejected(Rejected { issues })) if !issues.is_empty()
    ));
    let mut unmapped = other.clone();
    unmapped.model_version = "v2".into();
    unmapped.model_bundle = SALES.replace("warehouse", "archive").into_bytes();
    let status = client.publish_model(unmapped).await.unwrap_err();
    assert_eq!(status.code(), tonic::Code::FailedPrecondition);
    assert!(status.message().contains("archive"));
    let mut blank = other.clone();
    blank.model_version = " ".into();
    assert_eq!(
        client.publish_model(blank).await.unwrap_err().code(),
        tonic::Code::InvalidArgument
    );
    other.model_version = "v3".into();
    assert!(published(client.publish_model(other).await.unwrap().into_inner()).created);
    let listed = client
        .list_models(ListModelsRequest {})
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        listed
            .models
            .iter()
            .map(|m| m.model_version.as_str())
            .collect::<Vec<_>>(),
        ["v1", "v3"]
    );
    assert_eq!(listed.models[0].model_digest, digest);
    assert_eq!(listed.kernel_version, pp_cube_compile::KERNEL_VERSION);
    assert_eq!(listed.upstream_pin, pp_cube_compile::UPSTREAM_PIN);
    // The published version compiles under the identity that was returned.
    let compiled = client.compile(compile.clone()).await.unwrap().into_inner();
    assert_eq!(compiled.identity, Some(identity.clone()));
    assert!(matches!(
        compiled.result,
        Some(compile_response::Result::Compiled(_))
    ));
    let retired = client
        .retire_model(RetireModelRequest {
            model_version: "v1".into(),
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(retired.identity, Some(identity));
    assert_eq!(
        client.compile(compile.clone()).await.unwrap_err().code(),
        tonic::Code::NotFound
    );
    assert_eq!(
        client
            .retire_model(RetireModelRequest {
                model_version: "v1".into()
            })
            .await
            .unwrap_err()
            .code(),
        tonic::Code::NotFound
    );
    let mut v3 = compile;
    v3.model_version = "v3".into();
    assert!(matches!(
        client.compile(v3).await.unwrap().into_inner().result,
        Some(compile_response::Result::Compiled(value)) if value.sql.contains("dw.sales_v2")
    ));
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
}
