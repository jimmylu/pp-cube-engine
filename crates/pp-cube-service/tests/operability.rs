//! Access control, health, telemetry and limits over a real TCP connection.
use pp_cube_service::{
    grpc::{
        health::{
            health_check_response::ServingStatus, health_client::HealthClient,
            health_server::HealthServer, HealthCheckRequest,
        },
        wire::{cube_engine_client::CubeEngineClient, cube_engine_server::CubeEngineServer, *},
        Access, GrpcService, HealthService, SERVICE_NAME,
    },
    telemetry::Telemetry,
    CubeService,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tonic::{transport::Channel, Code, Request};

const SALES: &str = "cubes:\n- name: sales\n  data_source: warehouse\n  sql_table: dw.sales\n  measures:\n  - {name: amount, type: sum, sql: '{CUBE}.amount'}\n  dimensions:\n  - {name: region, type: string, sql: '{CUBE}.region'}\n";
const READ: &str = "reader-token-0123456789";
const ADMIN: &str = "admin-token-0123456789";

struct Host {
    client: CubeEngineClient<Channel>,
    health: HealthClient<Channel>,
    serving: Arc<AtomicBool>,
    stop: tokio::sync::oneshot::Sender<()>,
    server: tokio::task::JoinHandle<Result<(), tonic::transport::Error>>,
}
impl Host {
    async fn start(
        core: CubeService,
        max_request_bytes: usize,
        configure: impl FnOnce(GrpcService) -> GrpcService,
    ) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (stop, stopping) = tokio::sync::oneshot::channel();
        let serving = Arc::new(AtomicBool::new(true));
        let service = configure(GrpcService::new(Arc::new(core), 2, 4).unwrap());
        let server = tokio::spawn(
            tonic::transport::Server::builder()
                .add_service(HealthServer::new(HealthService::new(Arc::clone(&serving))))
                .add_service(
                    CubeEngineServer::new(service).max_decoding_message_size(max_request_bytes),
                )
                .serve_with_incoming_shutdown(
                    tokio_stream::wrappers::TcpListenerStream::new(listener),
                    async {
                        let _ = stopping.await;
                    },
                ),
        );
        let channel = Channel::from_shared(format!("http://{address}"))
            .unwrap()
            .connect()
            .await
            .unwrap();
        Self {
            client: CubeEngineClient::new(channel.clone()),
            health: HealthClient::new(channel),
            serving,
            stop,
            server,
        }
    }
    async fn finish(self) {
        self.stop.send(()).unwrap();
        self.server.await.unwrap().unwrap();
    }
}
fn core() -> CubeService {
    CubeService::new([("warehouse".to_owned(), pp_cube_compile::DialectId::Presto)])
}
fn with<T>(authorization: Option<&str>, message: T) -> Request<T> {
    let mut request = Request::new(message);
    if let Some(value) = authorization {
        request
            .metadata_mut()
            .insert("authorization", value.parse().unwrap());
    }
    request
}
fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}
fn compile() -> CompileRequest {
    CompileRequest {
        model_version: "v1".into(),
        query_json: r#"{"measures":["sales.amount"]}"#.into(),
        dialect: "presto".into(),
        timezone: "UTC".into(),
        ..Default::default()
    }
}
fn publish(version: &str, table: &str) -> PublishModelRequest {
    PublishModelRequest {
        model_version: version.into(),
        model_bundle: SALES.replace("dw.sales", table).into_bytes(),
        expected_digest: None,
    }
}

// scenario: B0-API-016
#[tokio::test(flavor = "current_thread")]
async fn tokens_separate_callers_from_administrators() {
    assert!(Access::new(Some("short"), None).is_err());
    assert!(Access::new(Some("has a space in 0123456789"), None).is_err());
    assert!(Access::new(Some(READ), Some(READ)).is_err());
    let access = Access::new(Some(READ), Some(ADMIN)).unwrap();
    let mut host = Host::start(core(), 1 << 20, |service| {
        service.with_management(true).with_access(access)
    })
    .await;
    let client = &mut host.client;
    // No credential, another scheme, an unknown token: not authenticated, for every operation.
    for authorization in [
        None,
        Some("Basic cmVhZGVyOnJlYWRlcg==".to_owned()),
        Some(bearer("reader-token-012345678X")),
        Some(bearer(&READ[..READ.len() - 1])),
        Some(READ.to_owned()),
    ] {
        let authorization = authorization.as_deref();
        assert_eq!(
            client
                .compile(with(authorization, compile()))
                .await
                .unwrap_err()
                .code(),
            Code::Unauthenticated,
            "{authorization:?}"
        );
        assert_eq!(
            client
                .publish_model(with(authorization, publish("v1", "dw.sales")))
                .await
                .unwrap_err()
                .code(),
            Code::Unauthenticated
        );
    }
    // The read token compiles, validates and reads metadata, but cannot manage versions.
    let read = bearer(READ);
    for refused in [
        client
            .publish_model(with(Some(&read), publish("v1", "dw.sales")))
            .await
            .unwrap_err(),
        client
            .list_models(with(Some(&read), ListModelsRequest {}))
            .await
            .unwrap_err(),
        client
            .retire_model(with(
                Some(&read),
                RetireModelRequest {
                    model_version: "v1".into(),
                },
            ))
            .await
            .unwrap_err(),
    ] {
        assert_eq!(refused.code(), Code::PermissionDenied);
    }
    let admin = bearer(ADMIN);
    client
        .publish_model(with(Some(&admin), publish("v1", "dw.sales")))
        .await
        .unwrap();
    for authorization in [&read, &admin] {
        let response = client
            .compile(with(Some(authorization), compile()))
            .await
            .unwrap()
            .into_inner();
        assert!(matches!(
            response.result,
            Some(compile_response::Result::Compiled(_))
        ));
    }
    client
        .get_meta(with(
            Some(&read),
            GetMetaRequest {
                model_version: "v1".into(),
            },
        ))
        .await
        .unwrap();
    client
        .validate_model(with(
            Some(&read),
            ValidateModelRequest {
                model_bundle: SALES.as_bytes().to_vec(),
            },
        ))
        .await
        .unwrap();
    // A probe needs no token.
    host.health
        .check(HealthCheckRequest::default())
        .await
        .unwrap();
    host.finish().await;

    // With a read token but no admin token, nobody was named as allowed to publish.
    let access = Access::new(Some(READ), None).unwrap();
    let mut host = Host::start(core(), 1 << 20, |service| {
        service.with_management(true).with_access(access)
    })
    .await;
    let status = host
        .client
        .publish_model(with(Some(&read), publish("v1", "dw.sales")))
        .await
        .unwrap_err();
    assert_eq!(status.code(), Code::PermissionDenied);
    host.finish().await;
}

// scenario: B0-API-017
#[tokio::test(flavor = "current_thread")]
async fn health_follows_the_standard_protocol_and_the_drain_flag() {
    let mut host = Host::start(core(), 1 << 20, |service| service).await;
    let check = |service: &str| HealthCheckRequest {
        service: service.into(),
    };
    for service in ["", SERVICE_NAME] {
        let status = host
            .health
            .check(check(service))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(status.status, ServingStatus::Serving as i32, "{service}");
    }
    assert_eq!(
        host.health
            .check(check("other.Service"))
            .await
            .unwrap_err()
            .code(),
        Code::NotFound
    );
    assert_eq!(
        host.health.watch(check("")).await.unwrap_err().code(),
        Code::Unimplemented
    );
    // A draining host says so while requests that are already running finish.
    host.serving.store(false, Ordering::Relaxed);
    let status = host.health.check(check("")).await.unwrap().into_inner();
    assert_eq!(status.status, ServingStatus::NotServing as i32);
    host.finish().await;
}

// scenario: B0-API-018
#[tokio::test(flavor = "current_thread")]
async fn every_request_is_counted_and_logged_without_its_content() {
    let lines = Arc::new(Mutex::new(Vec::<String>::new()));
    let sink = Arc::clone(&lines);
    let telemetry = Telemetry::default().with_log(move |line| sink.lock().unwrap().push(line));
    let metrics = Arc::clone(&telemetry.metrics);
    let access = Access::new(Some(READ), Some(ADMIN)).unwrap();
    let service_core = core();
    service_core
        .publish("v1", pp_cube_model::load_yaml(SALES).unwrap(), None)
        .unwrap();
    let mut host = Host::start(service_core, 1 << 20, |service| {
        service.with_access(access).with_telemetry(telemetry)
    })
    .await;
    let read = bearer(READ);
    let mut request = compile();
    request.request_id = format!("req-1\n{{\"forged\":true}}{}", "x".repeat(400));
    request.security_filters_json = vec![
        r#"{"member":"sales.region","operator":"equals","values":["north-secret-value"]}"#.into(),
    ];
    let compiled = host
        .client
        .compile(with(Some(&read), request))
        .await
        .unwrap()
        .into_inner();
    let Some(compile_response::Result::Compiled(compiled)) = compiled.result else {
        panic!("expected SQL")
    };
    let mut invalid = compile();
    invalid.query_json = r#"{"measures":["sales.missing"]}"#.into();
    host.client
        .compile(with(Some(&read), invalid))
        .await
        .unwrap();
    let mut unknown = compile();
    unknown.model_version = "missing".into();
    unknown.dialect = "made-up-dialect".into();
    host.client
        .compile(with(Some(&read), unknown))
        .await
        .unwrap_err();
    host.client
        .compile(with(None, compile()))
        .await
        .unwrap_err();
    host.finish().await;

    let lines = lines.lock().unwrap().clone();
    assert_eq!(lines.len(), 4, "one line per request: {lines:?}");
    let events: Vec<serde_json::Value> = lines
        .iter()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let outcomes: Vec<_> = events
        .iter()
        .map(|e| e["outcome"].as_str().unwrap())
        .collect();
    assert_eq!(outcomes, ["ok", "query", "NotFound", "Unauthenticated"]);
    for event in &events {
        assert_eq!(event["rpc"], "Compile");
        assert_eq!(event["event"], "request");
        assert!(event["duration_ms"].is_number() && event["ts_ms"].as_u64().unwrap() > 0);
    }
    assert_eq!(events[0]["model_version"], "v1");
    assert_eq!(events[0]["dialect"], "presto");
    let fingerprint: String = compiled
        .fingerprint
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(events[0]["fingerprint"], fingerprint.as_str());
    // Caller text is clipped and escaped; an unknown dialect is not echoed.
    assert_eq!(
        events[0]["request_id"].as_str().unwrap().chars().count(),
        128
    );
    assert!(events[0]["request_id"]
        .as_str()
        .unwrap()
        .starts_with("req-1\n{\"forged\""));
    assert!(events[0].get("forged").is_none());
    assert!(events[2].get("dialect").is_none());
    assert_eq!(events[2]["level"], "warn");
    for line in &lines {
        assert!(!line.contains('\n'));
        for secret in [
            "north-secret-value",
            "dw.sales",
            "SELECT",
            "sales.amount",
            READ,
            ADMIN,
        ] {
            assert!(!line.contains(secret), "{secret} in {line}");
        }
    }

    assert_eq!(metrics.requests("Compile", "ok"), 1);
    let text = metrics.render(1);
    for expected in [
        "ppcube_requests_total{rpc=\"Compile\",outcome=\"ok\"} 1\n",
        "ppcube_requests_total{rpc=\"Compile\",outcome=\"query\"} 1\n",
        "ppcube_requests_total{rpc=\"Compile\",outcome=\"NotFound\"} 1\n",
        "ppcube_requests_total{rpc=\"Compile\",outcome=\"Unauthenticated\"} 1\n",
        "ppcube_request_duration_seconds_bucket{rpc=\"Compile\",le=\"+Inf\"} 4\n",
        "ppcube_request_duration_seconds_count{rpc=\"Compile\"} 4\n",
        "ppcube_requests_in_flight 0\n",
        "ppcube_model_versions 1\n",
        "# TYPE ppcube_requests_total counter\n",
    ] {
        assert!(text.contains(expected), "{expected:?} missing from\n{text}");
    }
    assert!(text.contains(&format!(
        "ppcube_build_info{{kernel_version=\"{}\",upstream_pin=\"{}\"}} 1\n",
        pp_cube_compile::KERNEL_VERSION,
        pp_cube_compile::UPSTREAM_PIN
    )));
    for secret in ["north-secret-value", "dw.sales", "req-1", READ] {
        assert!(!text.contains(secret));
    }
}

// scenario: B0-API-019
#[tokio::test(flavor = "current_thread")]
async fn version_count_and_request_size_are_bounded() {
    let mut host = Host::start(core().with_max_versions(2), 2048, |service| {
        service.with_management(true)
    })
    .await;
    let client = &mut host.client;
    client.publish_model(publish("v1", "dw.a")).await.unwrap();
    client.publish_model(publish("v2", "dw.b")).await.unwrap();
    let status = client
        .publish_model(publish("v3", "dw.c"))
        .await
        .unwrap_err();
    assert_eq!(status.code(), Code::ResourceExhausted);
    // Republishing what is already there is not a new version, and retiring frees a place.
    client.publish_model(publish("v1", "dw.a")).await.unwrap();
    client
        .retire_model(RetireModelRequest {
            model_version: "v1".into(),
        })
        .await
        .unwrap();
    client.publish_model(publish("v3", "dw.c")).await.unwrap();
    let listed = client
        .list_models(ListModelsRequest {})
        .await
        .unwrap()
        .into_inner();
    assert_eq!(listed.models.len(), 2);
    // A request larger than the host's limit is refused before it is read as a model.
    let mut large = publish("v4", "dw.d");
    large.model_bundle.extend(std::iter::repeat_n(b' ', 4096));
    let status = client.publish_model(large).await.unwrap_err();
    assert_eq!(status.code(), Code::OutOfRange);
    assert_eq!(
        client
            .list_models(ListModelsRequest {})
            .await
            .unwrap()
            .into_inner()
            .models
            .len(),
        2
    );
    host.finish().await;
}
