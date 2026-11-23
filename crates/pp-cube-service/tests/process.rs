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
