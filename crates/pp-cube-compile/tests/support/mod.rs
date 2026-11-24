//! Replay of each successful golden compilation in four other processes.
//!
//! The four are started once for a test binary and kept: every compilation of
//! the binary is sent to each of them and must come back byte for byte as the
//! test process computed it. Starting them for every compilation made the
//! compile tests most of a check's time. Kept alive, they also show state that
//! leaks from one compilation into the next.
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};

use pp_cube_compile::{CompileContext, CompileError, Compiled, DialectId};
use pp_cube_model::{Model, Query};

const PREFIX: &str = "PP_COMPILED:";
/// How many other processes each compilation is repeated in.
const WORKERS: usize = 4;

fn record(result: &Compiled) -> String {
    format!("{PREFIX}{result:?}")
}

/// One other process: this test binary running only `support::worker`.
struct Worker {
    // Held so that the process is not reaped while its pipes are in use.
    _child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

fn start_worker() -> Child {
    Command::new(std::env::current_exe().unwrap())
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
        .expect("start a golden worker")
}

/// The workers of this test binary, started at the first compilation. They end
/// when the binary does: their input closes and their loop with it.
static POOL: Mutex<Vec<Worker>> = Mutex::new(Vec::new());

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
        DialectId::Doris => "doris",
    };
    // One line a compilation: the worker reads lines.
    let mut input = serde_json::to_string(&serde_json::json!({
        "model": model.as_ref(), "query": query, "dialect": dialect,
        "timezone": ctx.timezone, "security": ctx.security_filters,
        "request_id": ctx.request_id
    }))
    .unwrap();
    input.push('\n');
    // Tests of a binary run on several threads; the workers take one compilation
    // at a time. A test that failed while holding them leaves them usable.
    let mut pool = POOL.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    while pool.len() < WORKERS {
        let mut child = start_worker();
        let (input, output) = (child.stdin.take().unwrap(), child.stdout.take().unwrap());
        pool.push(Worker {
            _child: child,
            input,
            output: BufReader::new(output),
        });
    }
    for (run, worker) in pool.iter_mut().enumerate() {
        worker.input.write_all(input.as_bytes()).unwrap();
        worker.input.flush().unwrap();
        // libtest writes its own progress lines to the same pipe.
        let answered = loop {
            let mut line = String::new();
            let read = worker.output.read_line(&mut line).unwrap();
            assert!(
                read > 0,
                "worker {run} ended without a result for {query:?}"
            );
            if line.starts_with(PREFIX) {
                break line;
            }
        };
        assert_eq!(
            answered.trim_end(),
            expected,
            "process {run}, query {query:?}"
        );
    }
    Ok(result)
}

/// A worker given `input` and nothing more: what it wrote and how it ended.
fn run_worker(input: &[u8]) -> std::process::Output {
    let mut child = start_worker();
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
    // One compilation a line, until the test process closes the pipe.
    for line in std::io::stdin().lock().lines() {
        let mut input: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        let dialect = match input["dialect"].as_str().unwrap() {
            "presto" => DialectId::Presto,
            "doris" => DialectId::Doris,
            other => panic!("unknown dialect {other}"),
        };
        let model: Model = serde_json::from_value(input["model"].take()).unwrap();
        let query: Query = serde_json::from_value(input["query"].take()).unwrap();
        let ctx = CompileContext {
            dialect,
            timezone: input["timezone"].as_str().unwrap().into(),
            security_filters: serde_json::from_value(input["security"].take()).unwrap(),
            request_id: input["request_id"].as_str().unwrap().into(),
        };
        // Start a new line after libtest's single-thread progress prefix.
        println!(
            "\n{}",
            record(&pp_cube_compile::compile(&Arc::new(model), &query, &ctx).unwrap())
        );
        std::io::stdout().flush().unwrap();
    }
}

// scenario: B0-DET-001
#[test]
fn every_compilation_is_answered_alike_by_four_other_processes_that_stay() {
    let model = Arc::new(pp_cube_model::load_yaml(include_str!("../fixtures/sales.yaml")).unwrap());
    let ctx = CompileContext {
        dialect: DialectId::Presto,
        timezone: "UTC".into(),
        security_filters: vec![],
        request_id: String::new(),
    };
    let ids = || -> Vec<u32> {
        let pool = POOL.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        pool.iter().map(|worker| worker._child.id()).collect()
    };
    let first = pp_cube_model::load_query(r#"{"measures":["sales.amount"]}"#).unwrap();
    compile(&model, &first, &ctx).unwrap();
    let started = ids();
    assert_eq!(started.len(), WORKERS);
    assert!(started.iter().all(|id| *id != std::process::id()));
    // Other compilations, and the first again, go to the same four processes.
    let second = pp_cube_model::load_query(
        r#"{"measures":["sales.amount"],"dimensions":["sales.channel_cd"],"limit":3}"#,
    )
    .unwrap();
    for query in [&second, &first, &second] {
        compile(&model, query, &ctx).unwrap();
    }
    assert_eq!(ids(), started);
    // A compilation that fails is not sent: it has no result to compare.
    let unknown = pp_cube_model::load_query(r#"{"measures":["sales.missing"]}"#).unwrap();
    assert!(compile(&model, &unknown, &ctx).is_err());
    assert_eq!(ids(), started);
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
    let mut variants = vec![base.clone(); 6];
    variants[0].sql.push(' ');
    variants[1]
        .parameters
        .push(pp_cube_compile::Parameter::Null);
    variants[2].columns.clear();
    variants[3].used_cubes.clear();
    variants[4].fingerprint[0] ^= 1;
    variants[5]
        .session_properties
        .push(pp_cube_compile::SessionProperty {
            name: "offset_clause_enabled".into(),
            value: "true".into(),
        });
    for changed in variants {
        assert_ne!(record(&base), record(&changed));
    }
}
