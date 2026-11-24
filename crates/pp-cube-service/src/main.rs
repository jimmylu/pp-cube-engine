//! The service host: listener, TLS, tokens, limits, health, metrics and drain.
use pp_cube_service::{
    grpc::{
        health::health_server::HealthServer, wire::cube_engine_server::CubeEngineServer, Access,
        GrpcService, HealthService, Limits,
    },
    store::{log_lines, DirectoryStore, Scan, StoreSync},
    telemetry::Telemetry,
    CubeService,
};
use std::{
    collections::BTreeMap,
    future::Future,
    io::Write,
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

type Failure = Box<dyn std::error::Error>;

const VALUE_OPTIONS: [&str; 15] = [
    "--model-store",
    "--model-refresh-seconds",
    "--listen",
    "--workers",
    "--queue",
    "--tls-cert",
    "--tls-key",
    "--token-file",
    "--admin-token-file",
    "--metrics-listen",
    "--compile-timeout-ms",
    "--max-request-bytes",
    "--max-models",
    "--drain-seconds",
    "--log",
];

/// A token is the trimmed content of a file, so it never appears in a process listing.
fn token(path: Option<&String>) -> Result<Option<String>, Failure> {
    path.map(|path| Ok(std::fs::read_to_string(path)?.trim().to_owned()))
        .transpose()
}

/// Answers `GET /metrics` with the Prometheus text format and nothing else.
async fn serve_metrics(
    listener: tokio::net::TcpListener,
    telemetry: Telemetry,
    core: Arc<CubeService>,
    store: Option<Arc<StoreSync>>,
) {
    loop {
        let Ok((mut stream, _)) = listener.accept().await else {
            continue;
        };
        let (telemetry, core, store) = (telemetry.clone(), Arc::clone(&core), store.clone());
        tokio::spawn(async move {
            // A scrape is one small request; bound both its size and its time.
            let exchange = async {
                let mut request = Vec::new();
                let mut chunk = [0u8; 1024];
                while !request.windows(4).any(|w| w == b"\r\n\r\n") && request.len() < 8192 {
                    match stream.read(&mut chunk).await {
                        Ok(0) | Err(_) => break,
                        Ok(read) => request.extend_from_slice(&chunk[..read]),
                    }
                }
                let (status, body) = if request.starts_with(b"GET /metrics ") {
                    let mut body = telemetry.metrics.render(core.versions().len());
                    if let Some(store) = &store {
                        body.push_str(&store.render());
                    }
                    ("200 OK", body)
                } else {
                    ("404 Not Found", String::new())
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\ncontent-type: text/plain; version=0.0.4\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes()).await;
                let _ = stream.shutdown().await;
            };
            let _ = tokio::time::timeout(Duration::from_secs(5), exchange).await;
        });
    }
}

fn run() -> Result<(), Failure> {
    let mut args = std::env::args().skip(1);
    let mut options = BTreeMap::new();
    let mut data_sources = BTreeMap::new();
    let mut models = BTreeMap::new();
    let mut management = false;
    while let Some(key) = args.next() {
        if key == "--allow-publish" {
            management = true;
            continue;
        }
        if key == "--model" {
            // Repeatable: `--model v1=model.yaml`.
            let value = args.next().ok_or("missing option value")?;
            let (version, path) = value.split_once('=').ok_or("--model takes version=path")?;
            if version.trim().is_empty()
                || path.trim().is_empty()
                || models.insert(version.to_owned(), path.to_owned()).is_some()
            {
                return Err("blank or duplicate model version".into());
            }
            continue;
        }
        if key == "--data-source" {
            // Repeatable: `--data-source warehouse=doris`.
            let value = args.next().ok_or("missing option value")?;
            let (name, dialect) = value
                .split_once('=')
                .ok_or("--data-source takes name=dialect")?;
            let dialect = pp_cube_compile::DialectId::from_name(dialect)
                .ok_or("--data-source dialect must be presto or doris")?;
            if name.trim().is_empty() || data_sources.insert(name.to_owned(), dialect).is_some() {
                return Err("blank or duplicate data source".into());
            }
            continue;
        }
        if !VALUE_OPTIONS.contains(&key.as_str()) {
            return Err("unknown option; see crates/pp-cube-service/README.md".into());
        }
        let value = args.next().ok_or("missing option value")?;
        if value.trim().is_empty()
            || value.starts_with("--")
            || options.insert(key, value).is_some()
        {
            return Err("blank, missing or duplicate option value".into());
        }
    }
    let number = |name: &str, default: u64| -> Result<u64, Failure> {
        let value = options.get(name).map_or(Ok(default), |text| text.parse())?;
        if value == 0 {
            return Err(format!("{name} must be positive").into());
        }
        Ok(value)
    };
    let address: SocketAddr = options
        .get("--listen")
        .ok_or("required options: --model version=path, --listen, --data-source name=dialect")?
        .parse()?;
    let workers = number("--workers", 2)? as usize;
    let queue = number("--queue", 16)? as usize;
    let compile_timeout = Duration::from_millis(number("--compile-timeout-ms", 10_000)?);
    let max_request_bytes = number("--max-request-bytes", 4 * 1024 * 1024)? as usize;
    let max_models = number("--max-models", 64)? as usize;
    let drain = Duration::from_secs(number("--drain-seconds", 30)?);
    let log = match options.get("--log").map_or("json", String::as_str) {
        "json" => true,
        "off" => false,
        _ => return Err("--log is json or off".into()),
    };

    let tls = match (options.get("--tls-cert"), options.get("--tls-key")) {
        (Some(cert), Some(key)) => Some(tonic::transport::Identity::from_pem(
            std::fs::read(cert)?,
            std::fs::read(key)?,
        )),
        (None, None) => None,
        _ => return Err("--tls-cert and --tls-key go together".into()),
    };
    let access = Access::new(
        token(options.get("--token-file"))?.as_deref(),
        token(options.get("--admin-token-file"))?.as_deref(),
    )?;
    // Beyond loopback, the connection must be encrypted and every caller identified.
    if !address.ip().is_loopback() && (tls.is_none() || !options.contains_key("--token-file")) {
        return Err("a non-loopback --listen needs --tls-cert, --tls-key and --token-file".into());
    }
    if management && access.requires_token() && !access.has_admin_token() {
        return Err("--allow-publish with tokens needs --admin-token-file".into());
    }
    // Shared storage the semantic layer publishes to: a directory now; an
    // S3-protocol bucket is to be the second kind.
    let store = match options.get("--model-store") {
        Some(root) if root.contains("://") => {
            return Err("--model-store takes a directory; other stores are not built yet".into())
        }
        Some(root) => Some(Arc::new(StoreSync::new(DirectoryStore::new(root)))),
        None => None,
    };
    let refresh = match (options.get("--model-refresh-seconds"), &store) {
        (Some(_), None) => return Err("--model-refresh-seconds needs --model-store".into()),
        (seconds, _) => Duration::from_secs(seconds.map_or(Ok(30), |text| text.parse())?),
    };
    if models.is_empty() && !management && store.is_none() {
        return Err(
            "give at least one --model version=path, a --model-store, or --allow-publish".into(),
        );
    }
    let metrics_address: Option<SocketAddr> = options
        .get("--metrics-listen")
        .map(|text| text.parse())
        .transpose()?;

    let core = CubeService::new(data_sources).with_max_versions(max_models);
    for (version, path) in models {
        let text = std::fs::read_to_string(path)?;
        let model = if serde_json::from_str::<serde_json::Value>(&text).is_ok() {
            pp_cube_model::load_json(&text)?
        } else {
            pp_cube_model::load_yaml(&text)?
        };
        core.publish(version, model, None)?;
    }
    let core = Arc::new(core);
    let report = move |scan: &Scan| {
        if log {
            for line in log_lines(scan) {
                eprintln!("{line}");
            }
        }
    };
    // The first pass is part of starting: a store that cannot be listed stops the
    // process, as a missing model file does. A version that cannot be offered does
    // not: one bad publication must not keep the replicas from serving the others.
    if let Some(store) = &store {
        report(&store.scan(&core)?);
    }
    let telemetry = if log {
        Telemetry::default().with_log(|line| eprintln!("{line}"))
    } else {
        Telemetry::default()
    };
    let service = GrpcService::new(Arc::clone(&core), workers, queue)?
        .with_management(management)
        .with_access(access)
        .with_limits(Limits { compile_timeout })
        .with_telemetry(telemetry.clone());
    let serving = Arc::new(AtomicBool::new(true));
    let health = HealthService::new(Arc::clone(&serving));

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async move {
        // Install the handlers before announcing readiness, including an immediate signal.
        let mut interrupt = Box::pin(tokio::signal::ctrl_c());
        std::future::poll_fn(|cx| match interrupt.as_mut().poll(cx) {
            std::task::Poll::Pending => std::task::Poll::Ready(Ok(())),
            std::task::Poll::Ready(result) => std::task::Poll::Ready(result),
        })
        .await?;
        #[cfg(unix)]
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        let stop = async move {
            #[cfg(unix)]
            tokio::select! {
                _ = interrupt => {}
                _ = terminate.recv() => {}
            }
            #[cfg(not(unix))]
            let _ = interrupt.await;
        };

        let mut server = tonic::transport::Server::builder();
        if let Some(identity) = tls {
            server =
                server.tls_config(tonic::transport::ServerTlsConfig::new().identity(identity))?;
        }
        let listener = tokio::net::TcpListener::bind(address).await?;
        let metrics_listener = match metrics_address {
            Some(address) => Some(tokio::net::TcpListener::bind(address).await?),
            None => None,
        };
        println!("listening {}", listener.local_addr()?);
        if let Some(listener) = metrics_listener {
            println!("metrics {}", listener.local_addr()?);
            tokio::spawn(serve_metrics(
                listener,
                telemetry,
                Arc::clone(&core),
                store.clone(),
            ));
        }
        // Later passes pick up what the semantic layer publishes and retires. A pass
        // that cannot list the store changes nothing; the versions loaded stay.
        if let (Some(store), false) = (store, refresh.is_zero()) {
            let core = Arc::clone(&core);
            tokio::spawn(async move {
                let mut ticks = tokio::time::interval(refresh);
                ticks.tick().await;
                loop {
                    ticks.tick().await;
                    let (store, core) = (Arc::clone(&store), Arc::clone(&core));
                    let pass = tokio::task::spawn_blocking(move || store.scan(&core)).await;
                    match pass {
                        Ok(Ok(scan)) => report(&scan),
                        _ if log => eprintln!(
                            "{}",
                            serde_json::json!({"event": "model_store", "outcome": "scan-failed"})
                        ),
                        _ => {}
                    }
                }
            });
        }
        std::io::stdout().flush()?;

        // On a signal: report NOT_SERVING, accept nothing new, let running requests
        // finish, and give up when the drain time is over.
        let (draining, drained) = tokio::sync::oneshot::channel::<()>();
        let serve = server
            .add_service(HealthServer::new(health))
            .add_service(
                CubeEngineServer::new(service).max_decoding_message_size(max_request_bytes),
            )
            .serve_with_incoming_shutdown(
                tokio_stream::wrappers::TcpListenerStream::new(listener),
                async move {
                    stop.await;
                    serving.store(false, Ordering::Relaxed);
                    let _ = draining.send(());
                },
            );
        tokio::select! {
            result = serve => result?,
            _ = async {
                let _ = drained.await;
                tokio::time::sleep(drain).await;
            } => return Err("requests were still running when the drain time ended".into()),
        }
        Ok::<_, Failure>(())
    })
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}
