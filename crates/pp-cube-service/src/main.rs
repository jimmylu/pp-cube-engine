//! Local trusted host for one validated model version.
use pp_cube_service::{
    grpc::{wire::cube_engine_server::CubeEngineServer, GrpcService},
    CubeService,
};
use std::future::Future;
use std::{collections::BTreeMap, io::Write, net::SocketAddr, sync::Arc};

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let mut options = BTreeMap::new();
    let mut data_sources = BTreeMap::new();
    while let Some(key) = args.next() {
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
        if ![
            "--model",
            "--version",
            "--digest",
            "--listen",
            "--workers",
            "--queue",
        ]
        .contains(&key.as_str())
        {
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
    let required = |name| {
        options
            .get(name)
            .ok_or("required options: --model --version --digest --listen --data-source")
    };
    let address: SocketAddr = required("--listen")?.parse()?;
    if !address.ip().is_loopback() {
        return Err("listen address must be loopback".into());
    }
    let workers: usize = options
        .get("--workers")
        .map_or("2", String::as_str)
        .parse()?;
    let queue: usize = options
        .get("--queue")
        .map_or("16", String::as_str)
        .parse()?;
    let text = std::fs::read_to_string(required("--model")?)?;
    let model = if serde_json::from_str::<serde_json::Value>(&text).is_ok() {
        pp_cube_model::load_json(&text)?
    } else {
        pp_cube_model::load_yaml(&text)?
    };
    let mut core = CubeService::new(data_sources);
    core.register(
        required("--version")?.clone(),
        required("--digest")?.clone(),
        model,
    )?;
    let service = GrpcService::new(Arc::new(core), workers, queue)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async move {
        // Install the handler before announcing readiness, including an immediate SIGINT.
        let mut interrupt = Box::pin(tokio::signal::ctrl_c());
        std::future::poll_fn(|cx| match interrupt.as_mut().poll(cx) {
            std::task::Poll::Pending => std::task::Poll::Ready(Ok(())),
            std::task::Poll::Ready(result) => std::task::Poll::Ready(result),
        })
        .await?;
        let listener = tokio::net::TcpListener::bind(address).await?;
        println!("listening {}", listener.local_addr()?);
        std::io::stdout().flush()?;
        tonic::transport::Server::builder()
            .add_service(CubeEngineServer::new(service))
            .serve_with_incoming_shutdown(
                tokio_stream::wrappers::TcpListenerStream::new(listener),
                async move {
                    let _ = interrupt.await;
                },
            )
            .await?;
        Ok::<_, Box<dyn std::error::Error>>(())
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
