//! Four-process replay of each successful golden compilation.
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::Arc;

use pp_cube_compile::{CompileContext, CompileError, Compiled, DialectId};
use pp_cube_model::{Model, Query};

const PREFIX: &str = "PP_COMPILED:";

fn record(result: &Compiled) -> String {
    format!("{PREFIX}{result:?}")
}

// scenario: B0-DET-001
pub fn compile(
    model: &Arc<Model>,
    query: &Query,
    ctx: &CompileContext,
) -> Result<Compiled, CompileError> {
    let result = pp_cube_compile::compile(model, query, ctx)?;
    let expected = record(&result);
    let dialect = match ctx.dialect {
        DialectId::Presto => "presto",
    };
    for run in 0..4 {
        let input = serde_json::to_vec(&serde_json::json!({
            "model": model.as_ref(), "query": query, "dialect": dialect,
            "timezone": ctx.timezone, "security": ctx.security_filters,
            "request_id": ctx.request_id
        }))
        .unwrap();
        let output = run_worker(&input);
        assert!(
            output.status.success(),
            "worker {run}: {:?}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        let records: Vec<_> = stdout
            .lines()
            .filter(|line| line.starts_with(PREFIX))
            .collect();
        assert_eq!(
            records.len(),
            1,
            "worker {run} did not emit one result: {stdout}"
        );
        assert_eq!(records[0], expected, "process {run}, query {query:?}");
    }
    Ok(result)
}

fn run_worker(input: &[u8]) -> std::process::Output {
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "support::worker",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("PP_GOLDEN_WORKER", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start a fresh golden worker");
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

// scenario: B0-DET-001
#[test]
fn malformed_worker_input_fails_instead_of_emitting_a_result() {
    let output = run_worker(b"not json");
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout)
        .lines()
        .any(|s| s.starts_with(PREFIX)));
}

// scenario: B0-DET-001
#[test]
fn worker() {
    if std::env::var_os("PP_GOLDEN_WORKER").is_none() {
        return;
    }
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();
    let mut input: serde_json::Value = serde_json::from_str(&input).unwrap();
    assert_eq!(input["dialect"], "presto");
    let model: Model = serde_json::from_value(input["model"].take()).unwrap();
    let query: Query = serde_json::from_value(input["query"].take()).unwrap();
    let ctx = CompileContext {
        dialect: DialectId::Presto,
        timezone: input["timezone"].as_str().unwrap().into(),
        security_filters: serde_json::from_value(input["security"].take()).unwrap(),
        request_id: input["request_id"].as_str().unwrap().into(),
    };
    // Start a new line after libtest's single-thread progress prefix.
    println!(
        "\n{}",
        record(&pp_cube_compile::compile(&Arc::new(model), &query, &ctx).unwrap())
    );
}

// scenario: B0-DET-001
#[test]
fn comparison_includes_every_result_field() {
    let model = Arc::new(pp_cube_model::load_yaml(include_str!("../fixtures/sales.yaml")).unwrap());
    let query = pp_cube_model::load_query(r#"{"measures":["sales.amount"]}"#).unwrap();
    let ctx = CompileContext {
        dialect: DialectId::Presto,
        timezone: "UTC".into(),
        security_filters: vec![],
        request_id: String::new(),
    };
    let base = pp_cube_compile::compile(&model, &query, &ctx).unwrap();
    let mut variants = vec![base.clone(); 5];
    variants[0].sql.push(' ');
    variants[1]
        .parameters
        .push(pp_cube_compile::Parameter::Null);
    variants[2].columns.clear();
    variants[3].used_cubes.clear();
    variants[4].fingerprint[0] ^= 1;
    for changed in variants {
        assert_ne!(record(&base), record(&changed));
    }
}
