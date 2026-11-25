use pp_cube_service::grpc::wire::{cube_engine_client::CubeEngineClient, *};
use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
    time::Duration,
};

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_pp-cube-service"))
}
fn model() -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("pp-process-{}-{}.yaml", std::process::id(), {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }));
    std::fs::write(&path, "cubes:\n- name: sales\n  data_source: warehouse\n  sql_table: dw.sales\n  measures:\n  - {name: amount, type: sum, sql: '{CUBE}.amount'}\n").unwrap();
    path
}
// scenario: B0-API-009
#[test]
fn invalid_startup_never_announces_readiness() {
    for args in [
        vec![],
        vec!["--unknown", "x"],
        vec!["--model"],
        vec!["--model", "no-version-given"],
        vec!["--model", "=path"],
        vec!["--model", "v1="],
        // No model and no way to publish one.
        vec![
            "--listen",
            "127.0.0.1:0",
            "--data-source",
            "warehouse=presto",
        ],
    ] {
        let output = command().args(args).output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
    let path = model();
    let invalid = model();
    std::fs::write(&invalid, "cubes: [{name: invalid}]").unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    for (file, address, workers) in [
        (
            path.to_str().unwrap(),
            listener.local_addr().unwrap().to_string(),
            "2",
        ),
        (invalid.to_str().unwrap(), "127.0.0.1:0".into(), "2"),
        ("missing-model-file", "127.0.0.1:0".into(), "2"),
        (path.to_str().unwrap(), "0.0.0.0:5000".into(), "2"),
        (path.to_str().unwrap(), "127.0.0.1:0".into(), "0"),
    ] {
        let output = command()
            .args([
                "--model",
                &format!("v1={file}"),
                "--listen",
                &address,
                "--workers",
                workers,
                "--data-source",
                "warehouse=presto",
            ])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
    // The model's data source needs a dialect, and the option must be well formed.
    for sources in [
        vec![],
        vec!["--data-source", "lake=presto"],
        vec!["--data-source", "warehouse=oracle"],
        vec!["--data-source", "warehouse"],
        vec!["--data-source", "=presto"],
        vec![
            "--data-source",
            "warehouse=presto",
            "--data-source",
            "warehouse=doris",
        ],
    ] {
        let output = command()
            .args(["--model", &format!("v1={}", path.display())])
            .args(["--listen", "127.0.0.1:0"])
            .args(&sources)
            .output()
            .unwrap();
        assert!(!output.status.success(), "{sources:?}");
        assert!(output.stdout.is_empty(), "{sources:?}");
    }
    // The same version twice, and one invalid model among valid ones, start nothing.
    for models in [
        vec![
            format!("v1={}", path.display()),
            format!("v1={}", path.display()),
        ],
        vec![
            format!("v1={}", path.display()),
            format!("v2={}", invalid.display()),
        ],
    ] {
        let mut process = command();
        for model in &models {
            process.args(["--model", model]);
        }
        let output = process
            .args([
                "--listen",
                "127.0.0.1:0",
                "--data-source",
                "warehouse=presto",
            ])
            .output()
            .unwrap();
        assert!(!output.status.success(), "{models:?}");
        assert!(output.stdout.is_empty(), "{models:?}");
    }
    std::fs::remove_file(path).unwrap();
    std::fs::remove_file(invalid).unwrap();
}
// scenario: B0-API-008
#[cfg(unix)]
#[tokio::test(flavor = "current_thread")]
async fn real_process_serves_operations_and_exits_on_interrupt() {
    let path = model();
    // Exercise JSON loading, including surrogate-pair escapes in public metadata.
    std::fs::write(&path, r#"{"cubes":[{"name":"sales","data_source":"warehouse","title":"\ud83d\ude00","sql_table":"dw.sales","measures":[{"name":"amount","type":"sum","sql":"{CUBE}.amount"}]}]}"#).unwrap();
    let second = model();
    let mut child = Process(
        command()
            .args([
                "--model",
                &format!("v1={}", path.display()),
                "--model",
                &format!("v2={}", second.display()),
                "--allow-publish",
                "--listen",
                "127.0.0.1:0",
                "--data-source",
                "warehouse=presto",
                "--data-source",
                "lake=doris",
            ])
            .stdout(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let stdout = child.0.stdout.take().unwrap();
    let (send, receive) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        BufReader::new(stdout).read_line(&mut line).unwrap();
        let _ = send.send(line);
    });
    let line = receive.recv_timeout(Duration::from_secs(15)).unwrap();
    let address = line.trim().strip_prefix("listening ").unwrap();
    let mut client = CubeEngineClient::connect(format!("http://{address}"))
        .await
        .unwrap();
    client = client.max_decoding_message_size(1024 * 1024);
    let response = client
        .compile(CompileRequest {
            model_version: "v1".into(),
            query_json: r#"{"measures":["sales.amount"]}"#.into(),
            dialect: "presto".into(),
            timezone: "UTC".into(),
            ..Default::default()
        })
        .await
        .unwrap()
        .into_inner();
    assert!(matches!(
        response.result,
        Some(compile_response::Result::Compiled(_))
    ));
    assert!(!client
        .get_meta(GetMetaRequest {
            model_version: "v1".into()
        })
        .await
        .unwrap()
        .into_inner()
        .cubes
        .is_empty());
    // Both startup versions are available, and publishing was enabled by the flag.
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
        ["v1", "v2"]
    );
    assert!(listed
        .models
        .iter()
        .all(|m| m.model_digest.starts_with("sha256:")));
    assert!(!client
        .validate_model(ValidateModelRequest {
            model_bundle: b"cubes: [{name: invalid}]".to_vec()
        })
        .await
        .unwrap()
        .into_inner()
        .issues
        .is_empty());
    assert_eq!(
        client
            .get_meta(GetMetaRequest {
                model_version: "missing".into()
            })
            .await
            .unwrap_err()
            .code(),
        tonic::Code::NotFound
    );
    drop(client);
    assert!(Command::new("kill")
        .args(["-INT", &child.0.id().to_string()])
        .status()
        .unwrap()
        .success());
    let status = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Some(status) = child.0.try_wait().unwrap() {
                break status;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    assert!(status.success());
    std::fs::remove_file(path).unwrap();
    std::fs::remove_file(second).unwrap();
}

fn scratch(name: &str) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    std::env::temp_dir().join(format!(
        "pp-host-{}-{}-{name}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ))
}
/// A self-signed certificate for `localhost`, made by the system's openssl.
fn certificate() -> (std::path::PathBuf, std::path::PathBuf) {
    let (cert, key, config) = (
        scratch("cert.pem"),
        scratch("key.pem"),
        scratch("openssl.cnf"),
    );
    // An end-entity certificate: rustls refuses a CA certificate presented by a server.
    std::fs::write(
        &config,
        "[req]\ndistinguished_name=dn\nx509_extensions=ext\nprompt=no\n[dn]\nCN=localhost\n\
         [ext]\nsubjectAltName=DNS:localhost,IP:127.0.0.1\nbasicConstraints=critical,CA:FALSE\n",
    )
    .unwrap();
    let made = Command::new("openssl")
        .args([
            "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "2",
        ])
        .arg("-config")
        .arg(&config)
        .arg("-keyout")
        .arg(&key)
        .arg("-out")
        .arg(&cert)
        .output()
        .expect("openssl is needed to make a test certificate");
    assert!(
        made.status.success(),
        "{}",
        String::from_utf8_lossy(&made.stderr)
    );
    std::fs::remove_file(config).unwrap();
    (cert, key)
}
const READ: &str = "reader-token-0123456789";
const ADMIN: &str = "admin-token-0123456789";

// scenario: B0-API-020
#[test]
fn a_host_beyond_loopback_must_be_encrypted_and_authenticated() {
    let path = model();
    let (cert, key) = certificate();
    let (read, short) = (scratch("read"), scratch("short"));
    std::fs::write(&read, format!("{READ}\n")).unwrap();
    std::fs::write(&short, "short").unwrap();
    let (cert, key, read, short) = (
        cert.to_str().unwrap(),
        key.to_str().unwrap(),
        read.to_str().unwrap(),
        short.to_str().unwrap(),
    );
    for (listen, extra) in [
        ("0.0.0.0:0", vec![]),
        ("0.0.0.0:0", vec!["--tls-cert", cert, "--tls-key", key]),
        ("0.0.0.0:0", vec!["--token-file", read]),
        ("127.0.0.1:0", vec!["--tls-cert", cert]),
        ("127.0.0.1:0", vec!["--tls-cert", read, "--tls-key", key]),
        ("127.0.0.1:0", vec!["--token-file", short]),
        ("127.0.0.1:0", vec!["--token-file", "missing-token-file"]),
        (
            "127.0.0.1:0",
            vec!["--token-file", read, "--admin-token-file", read],
        ),
        // Tokens in use, publishing enabled, and nobody named as allowed to publish.
        ("127.0.0.1:0", vec!["--token-file", read, "--allow-publish"]),
        ("127.0.0.1:0", vec!["--log", "xml"]),
        ("127.0.0.1:0", vec!["--max-models", "0"]),
        ("127.0.0.1:0", vec!["--compile-timeout-ms", "soon"]),
        ("127.0.0.1:0", vec!["--metrics-listen", "nowhere"]),
    ] {
        let output = command()
            .args(["--model", &format!("v1={}", path.display())])
            .args(["--data-source", "warehouse=presto", "--listen", listen])
            .args(&extra)
            .output()
            .unwrap();
        assert!(!output.status.success(), "{listen} {extra:?}");
        assert!(output.stdout.is_empty(), "{listen} {extra:?}");
        assert!(!output.stderr.is_empty(), "{listen} {extra:?}");
    }
    for file in [path.to_str().unwrap(), cert, key, read, short] {
        std::fs::remove_file(file).unwrap();
    }
}

// scenario: B0-API-020
#[cfg(unix)]
#[tokio::test(flavor = "current_thread")]
async fn an_encrypted_host_serves_probes_metrics_and_drains_on_terminate() {
    use pp_cube_service::grpc::health::{
        health_check_response::ServingStatus, health_client::HealthClient, HealthCheckRequest,
    };
    use std::io::{Read, Write};
    use tonic::transport::{Certificate, Channel, ClientTlsConfig};
    let path = model();
    let (cert, key) = certificate();
    let (read, admin) = (scratch("read"), scratch("admin"));
    std::fs::write(&read, READ).unwrap();
    std::fs::write(&admin, format!("{ADMIN}\n")).unwrap();
    let mut child = Process(
        command()
            .args(["--model", &format!("v1={}", path.display())])
            .args(["--data-source", "warehouse=presto", "--listen", "0.0.0.0:0"])
            .args(["--metrics-listen", "127.0.0.1:0", "--allow-publish"])
            .arg("--tls-cert")
            .arg(&cert)
            .arg("--tls-key")
            .arg(&key)
            .arg("--token-file")
            .arg(&read)
            .arg("--admin-token-file")
            .arg(&admin)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let stdout = child.0.stdout.take().unwrap();
    let (send, receive) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        for _ in 0..2 {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let _ = send.send(line);
        }
    });
    let announced = |prefix: &str| {
        let line = receive.recv_timeout(Duration::from_secs(15)).unwrap();
        let address = line.trim().strip_prefix(prefix).unwrap().to_owned();
        address.rsplit(':').next().unwrap().to_owned()
    };
    let port = announced("listening ");
    let metrics_port = announced("metrics ");

    let tls = ClientTlsConfig::new()
        .ca_certificate(Certificate::from_pem(std::fs::read(&cert).unwrap()))
        .domain_name("localhost");
    let channel = Channel::from_shared(format!("https://127.0.0.1:{port}"))
        .unwrap()
        .tls_config(tls)
        .unwrap()
        .connect()
        .await
        .unwrap();
    let mut client = CubeEngineClient::new(channel.clone());
    let request = |token: Option<&str>| {
        let mut request = tonic::Request::new(CompileRequest {
            model_version: "v1".into(),
            query_json: r#"{"measures":["sales.amount"]}"#.into(),
            dialect: "presto".into(),
            timezone: "UTC".into(),
            request_id: "tls-test".into(),
            ..Default::default()
        });
        if let Some(token) = token {
            let value = format!("Bearer {token}").parse().unwrap();
            request.metadata_mut().insert("authorization", value);
        }
        request
    };
    // Encrypted and identified callers are served; an unidentified one is not.
    assert!(matches!(
        client
            .compile(request(Some(READ)))
            .await
            .unwrap()
            .into_inner()
            .result,
        Some(compile_response::Result::Compiled(_))
    ));
    assert_eq!(
        client.compile(request(None)).await.unwrap_err().code(),
        tonic::Code::Unauthenticated
    );
    let mut listing = tonic::Request::new(ListModelsRequest {});
    let value = format!("Bearer {ADMIN}").parse().unwrap();
    listing.metadata_mut().insert("authorization", value);
    assert_eq!(
        client
            .list_models(listing)
            .await
            .unwrap()
            .into_inner()
            .models
            .len(),
        1
    );
    let status = HealthClient::new(channel)
        .check(HealthCheckRequest::default())
        .await
        .unwrap()
        .into_inner();
    assert_eq!(status.status, ServingStatus::Serving as i32);
    // The same port refuses a caller that does not speak TLS.
    let plain = async {
        CubeEngineClient::connect(format!("http://127.0.0.1:{port}"))
            .await?
            .compile(request(Some(READ)))
            .await
            .map_err(|status| Box::new(status) as Box<dyn std::error::Error>)
    };
    assert!(plain.await.is_err());

    let scrape = move |path: &str| {
        let mut stream = std::net::TcpStream::connect(format!("127.0.0.1:{metrics_port}")).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        write!(stream, "GET {path} HTTP/1.1\r\nhost: localhost\r\n\r\n").unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response
    };
    // The scrape runs on the host's single runtime thread; keep this one off ours.
    let metrics = tokio::task::spawn_blocking(move || (scrape("/metrics"), scrape("/secret")))
        .await
        .unwrap();
    assert!(
        metrics.0.starts_with("HTTP/1.1 200 OK\r\n"),
        "{}",
        metrics.0
    );
    assert!(metrics
        .0
        .contains("ppcube_requests_total{rpc=\"Compile\",outcome=\"ok\"} 1\n"));
    assert!(metrics
        .0
        .contains("ppcube_requests_total{rpc=\"Compile\",outcome=\"Unauthenticated\"} 1\n"));
    assert!(metrics.0.contains("ppcube_model_versions 1\n"));
    assert!(metrics.1.starts_with("HTTP/1.1 404 "), "{}", metrics.1);

    drop(client);
    // Orchestrators stop a container with SIGTERM.
    assert!(Command::new("kill")
        .args(["-TERM", &child.0.id().to_string()])
        .status()
        .unwrap()
        .success());
    let status = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Some(status) = child.0.try_wait().unwrap() {
                break status;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    assert!(status.success());
    let mut log = String::new();
    child
        .0
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut log)
        .unwrap();
    let events: Vec<serde_json::Value> = log
        .lines()
        .map(|line| serde_json::from_str(line).expect("every stderr line is a JSON event"))
        .collect();
    assert!(events
        .iter()
        .any(|e| e["rpc"] == "Compile" && e["outcome"] == "ok" && e["request_id"] == "tls-test"));
    assert!(!log.contains(READ) && !log.contains(ADMIN) && !log.contains("dw.sales"));
    for file in [&path, &cert, &key, &read, &admin] {
        std::fs::remove_file(file).unwrap();
    }
}

// scenario: B1-API-002
#[tokio::test(flavor = "current_thread")]
async fn a_replica_serves_what_the_store_holds_and_follows_it_while_running() {
    let root = scratch("store");
    let write = |key: &str, text: &str| {
        let path = key
            .split('/')
            .fold(root.clone(), |path, part| path.join(part));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    let cube = "cubes:\n- name: sales\n  data_source: warehouse\n  sql_table: dw.sales\n  measures:\n  - {name: amount, type: sum, sql: '{CUBE}.amount'}\n";
    write("property/motor/models/v1/cubes/sales.yml", cube);
    write("property/motor/models/v1/manifest.json", r#"{"schema": 1}"#);
    // A publication that is not a model must not keep the replica from starting.
    write("property/motor/models/broken/cubes/sales.yml", "cubes: [");
    write(
        "property/motor/models/broken/manifest.json",
        r#"{"schema": 1}"#,
    );

    // A store that is not there, or of a kind not built, stops the process.
    for store in ["/nonexistent/pp-model-store", "s3://bucket/prefix"] {
        let output = command()
            .args(["--model-store", store, "--listen", "127.0.0.1:0"])
            .args(["--data-source", "warehouse=presto"])
            .output()
            .unwrap();
        assert!(
            !output.status.success() && output.stdout.is_empty(),
            "{store}"
        );
    }
    let output = command()
        .args(["--model-refresh-seconds", "5", "--allow-publish"])
        .args([
            "--listen",
            "127.0.0.1:0",
            "--data-source",
            "warehouse=presto",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success() && output.stdout.is_empty());

    let mut child = Process(
        command()
            .args(["--model-store", root.to_str().unwrap()])
            .args(["--model-refresh-seconds", "1"])
            .args([
                "--listen",
                "127.0.0.1:0",
                "--data-source",
                "warehouse=presto",
            ])
            .stdout(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let stdout = child.0.stdout.take().unwrap();
    let (send, receive) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        BufReader::new(stdout).read_line(&mut line).unwrap();
        let _ = send.send(line);
    });
    let line = receive.recv_timeout(Duration::from_secs(15)).unwrap();
    let address = line.trim().strip_prefix("listening ").unwrap().to_owned();
    let mut client = CubeEngineClient::connect(format!("http://{address}"))
        .await
        .unwrap();
    // Whether a version compiles now, asked of the running process.
    async fn compiles(
        client: &mut CubeEngineClient<tonic::transport::Channel>,
        version: &str,
    ) -> bool {
        let request = CompileRequest {
            model_version: version.into(),
            query_json: r#"{"measures":["sales.amount"]}"#.into(),
            dialect: "presto".into(),
            timezone: "UTC".into(),
            ..Default::default()
        };
        match client.compile(request).await {
            Ok(response) => matches!(
                response.into_inner().result,
                Some(compile_response::Result::Compiled(_))
            ),
            Err(_) => false,
        }
    }
    // Loaded before it announced itself: the version of the store compiles at once.
    assert!(compiles(&mut client, "property/motor/v1").await);
    assert!(!compiles(&mut client, "property/motor/broken").await);
    assert!(!compiles(&mut client, "property/motor/v2").await);

    // Published while it runs: files first, the manifest last.
    write("property/motor/models/v2/cubes/sales.yml", cube);
    write("property/motor/models/v2/manifest.json", r#"{"schema": 1}"#);
    let mut seen = false;
    for _ in 0..100 {
        if compiles(&mut client, "property/motor/v2").await {
            seen = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(seen, "the version published to the store was not picked up");

    // Retired by removing its manifest.
    std::fs::remove_file(root.join("property/motor/models/v1/manifest.json")).unwrap();
    let mut gone = false;
    for _ in 0..100 {
        if !compiles(&mut client, "property/motor/v1").await {
            gone = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(gone, "the version retired in the store is still offered");
    assert!(compiles(&mut client, "property/motor/v2").await);
    let _ = std::fs::remove_dir_all(root);
}

/// The arguments of the service container in the shipped Kubernetes manifest,
/// and the manifest's text. The list is one quoted string a line.
fn deployment() -> (Vec<String>, String) {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../deploy/kubernetes.yaml");
    let text = std::fs::read_to_string(path).expect("the deployment manifest is in the repository");
    let lines = text
        .lines()
        .skip_while(|line| line.trim() != "args:")
        .skip(1);
    let arguments = lines
        .map_while(|line| line.trim().strip_prefix("- \""))
        .map(|rest| {
            rest.strip_suffix('"')
                .expect("a quoted argument")
                .to_owned()
        })
        .collect();
    (arguments, text)
}

// scenario: B1-API-004
#[cfg(unix)]
#[tokio::test(flavor = "current_thread")]
async fn the_shipped_deployment_starts_replicas_that_serve_and_answer_the_probe() {
    use std::io::{Read, Write};
    let (arguments, manifest) = deployment();
    // Three replicas, replaced one at a time, never fewer than two.
    for stated in [
        "  replicas: 3\n",
        "      maxUnavailable: 1\n",
        "  minAvailable: 2\n",
        "kind: PodDisruptionBudget\n",
        "kind: Service\n",
    ] {
        assert!(manifest.contains(stated), "{stated}");
    }
    // What the container is given: its secrets and the store, read-only, as
    // a user that is not root. A version is named, never `latest`.
    for stated in [
        "runAsNonRoot: true",
        "readOnlyRootFilesystem: true",
        "image: ghcr.io/jimmylu/pp-cube-engine:",
    ] {
        assert!(manifest.contains(stated), "{stated}");
    }
    assert!(!manifest.contains(":latest"));
    let image = manifest
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("image: ghcr.io/jimmylu/pp-cube-engine:")
        })
        .unwrap();
    assert_eq!(
        image,
        env!("CARGO_PKG_VERSION"),
        "the manifest names this version"
    );
    // The probe asks the metrics port, and the pod outlives the drain.
    let value = |flag: &str| {
        let at = arguments
            .iter()
            .position(|a| a == flag)
            .unwrap_or_else(|| panic!("{flag}"));
        arguments[at + 1].clone()
    };
    assert_eq!(value("--metrics-listen"), "0.0.0.0:9100");
    assert!(manifest.contains("path: /metrics") && manifest.contains("containerPort: 9100"));
    let drain: u64 = value("--drain-seconds").parse().unwrap();
    let grace: u64 = manifest
        .lines()
        .find_map(|line| line.trim().strip_prefix("terminationGracePeriodSeconds: "))
        .unwrap()
        .parse()
        .unwrap();
    assert!(grace > drain, "{grace} > {drain}");

    // The same arguments start the service. Only where they point changes: the
    // mounted files are made here, and the ports are chosen by the system.
    let (cert, key) = certificate();
    let (read, admin, store) = (scratch("read"), scratch("admin"), scratch("models"));
    std::fs::write(&read, READ).unwrap();
    std::fs::write(&admin, ADMIN).unwrap();
    let version = store
        .join("property")
        .join("motor")
        .join("models")
        .join("v1");
    std::fs::create_dir_all(version.join("cubes")).unwrap();
    std::fs::write(
        version.join("cubes").join("sales.yml"),
        "cubes:\n- name: sales\n  data_source: warehouse\n  sql_table: dw.sales\n  measures:\n  - {name: amount, type: sum, sql: '{CUBE}.amount'}\n",
    )
    .unwrap();
    std::fs::write(version.join("manifest.json"), r#"{"schema": 1}"#).unwrap();
    let placed: Vec<String> = arguments
        .iter()
        .map(|argument| match argument.as_str() {
            "/etc/pp-cube-engine/tls.crt" => cert.display().to_string(),
            "/etc/pp-cube-engine/tls.key" => key.display().to_string(),
            "/etc/pp-cube-engine/read.token" => read.display().to_string(),
            "/etc/pp-cube-engine/admin.token" => admin.display().to_string(),
            "/var/lib/pp-cube-engine/models" => store.display().to_string(),
            "0.0.0.0:50051" => "0.0.0.0:0".to_owned(),
            "0.0.0.0:9100" => "127.0.0.1:0".to_owned(),
            other => other.to_owned(),
        })
        .collect();
    // Every path of the manifest was one this test knows and made.
    assert!(!placed
        .iter()
        .any(|a| a.starts_with("/etc/pp-cube-engine") || a.starts_with("/var/lib/pp-cube-engine")));
    let mut child = Process(
        command()
            .args(&placed)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let stdout = child.0.stdout.take().unwrap();
    let (send, receive) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        for _ in 0..2 {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let _ = send.send(line);
        }
    });
    let announced = |prefix: &str| {
        let line = receive
            .recv_timeout(Duration::from_secs(15))
            .expect("the service starts with the manifest's arguments");
        let address = line.trim().strip_prefix(prefix).unwrap().to_owned();
        address.rsplit(':').next().unwrap().to_owned()
    };
    let _port = announced("listening ");
    let metrics_port = announced("metrics ");
    // What the readiness probe asks, and what it shows: the store's version is served.
    let probe = tokio::task::spawn_blocking(move || {
        let mut stream = std::net::TcpStream::connect(format!("127.0.0.1:{metrics_port}")).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        write!(stream, "GET /metrics HTTP/1.1\r\nhost: localhost\r\n\r\n").unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response
    })
    .await
    .unwrap();
    assert!(probe.starts_with("HTTP/1.1 200 OK\r\n"), "{probe}");
    assert!(probe.contains("ppcube_model_versions 1\n"), "{probe}");
    // Kubernetes stops a pod with SIGTERM; the replica drains and exits cleanly.
    let pid = child.0.id().to_string();
    assert!(Command::new("kill")
        .args(["-TERM", &pid])
        .status()
        .unwrap()
        .success());
    assert!(child.0.wait().unwrap().success());
}
