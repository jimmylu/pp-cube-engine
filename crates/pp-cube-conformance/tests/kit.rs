//! The integration test kit under `tools/integration-kit` and its guide are
//! used by hand, on engines no check here reaches. These keep what they rest
//! on true without Python, a service or an engine: the benchmark rows are the
//! fixture's, the suite expects of each dialect what the kernel does, and the
//! guide names only commands the kit has.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use pp_cube_compile::{compile, CompileContext, CompileError, DialectId};
use pp_cube_model::{assemble, load_query, ModelFile};
use serde_json::Value;

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &str) -> String {
    let path = repository().join(path);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn json(path: &str) -> Value {
    serde_json::from_str(&read(path)).unwrap_or_else(|error| panic!("{path}: {error}"))
}

const EXAMPLE: &str = "examples/insurance-analytics";

/// The example's model as the kit publishes it for a dialect. Table names do
/// not decide what compiles; on Druid the time column does.
fn model(dialect: DialectId, time_columns: &[String]) -> pp_cube_model::Model {
    let version = repository()
        .join(EXAMPLE)
        .join("model-store/property/models/2026-11");
    let mut files = Vec::new();
    for folder in ["cubes", "views"] {
        let mut paths: Vec<_> = std::fs::read_dir(version.join(folder))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        paths.sort();
        for path in paths {
            let mut text = std::fs::read_to_string(&path).unwrap();
            if dialect == DialectId::Druid {
                text = text.replace("dw.", "druid.");
                for column in time_columns {
                    text = text.replace(&format!("{{CUBE}}.{column}\""), "{CUBE}.__time\"");
                }
            }
            let name = path.file_name().unwrap().to_str().unwrap().to_owned();
            files.push((format!("{folder}/{name}"), text));
        }
    }
    let files: Vec<ModelFile<'_>> = files
        .iter()
        .map(|(path, text)| ModelFile { path, text })
        .collect();
    assemble(&files).expect("the example's model assembles")
}

// scenario: B1-KIT-001
#[test]
fn the_benchmark_rows_are_the_fixtures_and_the_suite_expects_what_each_dialect_does() {
    // The rows the kit loads are the rows the expected answers were worked out from.
    let data = json(&format!("{EXAMPLE}/warehouse/insurance.json"));
    let manifest: Value =
        serde_json::from_str(&pp_cube_conformance::manifest("presto").unwrap()).unwrap();
    let cases = manifest["cases"].as_array().unwrap();
    let fixture: Vec<&str> = cases
        .iter()
        .find(|case| case["name"] == "x_totals")
        .unwrap()["fixture"]
        .as_array()
        .unwrap()
        .iter()
        .map(|statement| statement.as_str().unwrap())
        .collect();
    let presto_type = |kind: &str| match kind {
        "integer" => "INTEGER",
        "string" => "VARCHAR",
        "double" => "DOUBLE",
        "timestamp" => "TIMESTAMP",
        other => panic!("unknown column type {other}"),
    };
    let tables = data["tables"].as_array().unwrap();
    assert_eq!(tables.len(), 9);
    let mut time_columns = Vec::new();
    for table in tables {
        let name = table["name"].as_str().unwrap();
        assert!(name.starts_with("ins_"), "{name}");
        let columns = table["columns"].as_array().unwrap();
        let declared: Vec<String> = columns
            .iter()
            .map(|c| {
                format!(
                    "{} {}",
                    c["name"].as_str().unwrap(),
                    presto_type(c["type"].as_str().unwrap())
                )
            })
            .collect();
        let create = format!("CREATE TABLE memory.dw.{name} ({})", declared.join(", "));
        assert!(fixture.contains(&create.as_str()), "{create}");
        let rows: Vec<String> = table["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                let cells: Vec<String> = row
                    .as_array()
                    .unwrap()
                    .iter()
                    .zip(columns)
                    .map(
                        |(cell, column)| match (cell, column["type"].as_str().unwrap()) {
                            (Value::Null, _) => "NULL".to_owned(),
                            (Value::String(text), "timestamp") => {
                                format!("TIMESTAMP '{}'", text.replace('T', " "))
                            }
                            (Value::String(text), _) => format!("'{}'", text.replace('\'', "''")),
                            (number, _) => number.to_string(),
                        },
                    )
                    .collect();
                format!("({})", cells.join(","))
            })
            .collect();
        let insert = format!("INSERT INTO memory.dw.{name} VALUES {}", rows.join(","));
        assert!(
            fixture.contains(&insert.as_str()),
            "{name}: its rows are not the fixture's"
        );
        if let Some(time) = table.get("time").and_then(Value::as_str) {
            assert!(
                columns
                    .iter()
                    .any(|c| c["name"] == time && c["type"] == "timestamp"),
                "{name}"
            );
            time_columns.push(time.to_owned());
        }
    }
    // The Presto statements beside it are the same fixture, one a line.
    let sql = read(&format!("{EXAMPLE}/warehouse/insurance.sql"));
    let lines: Vec<&str> = sql
        .lines()
        .filter(|line| !line.starts_with("--") && !line.is_empty())
        .map(|line| line.strip_suffix(';').expect("one statement a line"))
        .collect();
    assert_eq!(lines, fixture);

    // What the built-in suite expects of each dialect is what the kernel does there.
    let portal = json(&format!("{EXAMPLE}/portal.json"));
    let differences = json("tools/integration-kit/suites/insurance-dialects.json");
    let mut summary = Vec::new();
    for (name, dialect) in [
        ("presto", DialectId::Presto),
        ("doris", DialectId::Doris),
        ("druid", DialectId::Druid),
    ] {
        let model = Arc::new(model(dialect, &time_columns));
        let differs = differences.get(name).cloned().unwrap_or(Value::Null);
        let (mut compiled_count, mut refused_count) = (0, 0);
        let mut named = BTreeSet::new();
        for user in portal["users"].as_array().unwrap() {
            let rules = serde_json::json!({ "filters": user["rowFilters"] }).to_string();
            let security_filters = load_query(&rules).unwrap().filters;
            for question in user["questions"].as_array().unwrap() {
                let case = format!(
                    "{}/{}",
                    user["name"].as_str().unwrap(),
                    question["name"].as_str().unwrap()
                );
                let stated = differs.get(&case).unwrap_or(question);
                named.insert(case.clone());
                let context = CompileContext {
                    request_id: String::new(),
                    dialect,
                    timezone: "UTC".into(),
                    security_filters: security_filters.clone(),
                };
                let query = load_query(&question["query"].to_string()).unwrap();
                match (compile(&model, &query, &context), stated.get("refused")) {
                    (Ok(compiled), None) => {
                        if let Some(expect) = stated.get("expect") {
                            let aliases: Vec<&str> =
                                compiled.columns.iter().map(|c| c.alias.as_str()).collect();
                            assert_eq!(
                                expect["columns"],
                                serde_json::json!(aliases),
                                "{name} {case}"
                            );
                        }
                        compiled_count += 1;
                    }
                    (Err(CompileError::Unsupported(_)), Some(refused)) => {
                        assert_eq!(refused["kind"], "UNSUPPORTED", "{name} {case}");
                        refused_count += 1;
                    }
                    (Err(CompileError::Query(_)), Some(refused)) => {
                        assert_eq!(refused["kind"], "QUERY", "{name} {case}");
                        refused_count += 1;
                    }
                    (outcome, expected) => panic!(
                        "{name} {case}: the suite expects {expected:?}, the kernel gives {:?}",
                        outcome.map(|compiled| compiled.sql)
                    ),
                }
            }
        }
        // A difference is stated for a question the example has.
        if let Some(stated) = differs.as_object() {
            for case in stated.keys() {
                assert!(named.contains(case), "{name}: no question {case}");
            }
        }
        summary.push((name, compiled_count, refused_count));
    }
    // The figures the guide gives its reader.
    assert_eq!(
        summary,
        [("presto", 15, 2), ("doris", 16, 1), ("druid", 15, 2)]
    );
    let guide = read("docs/integration-testing.zh-CN.md");
    for expected in [
        "| Presto 0.296 | 15 个 `PASS`，2 个 `REFUSED` |",
        "| Doris 2.1.8 | 16 个 `PASS`，1 个 `REFUSED` |",
        "| Druid 26.0 | 15 个 `PASS`，2 个 `REFUSED` |",
    ] {
        assert!(guide.contains(expected), "{expected}");
    }
}

// scenario: B1-KIT-002
#[test]
fn the_guide_names_only_commands_and_options_the_kit_has() {
    let cli = read("tools/integration-kit/src/ppcube_itest/cli.py");
    // The commands, as the kit registers them.
    let mut commands = BTreeSet::new();
    for marker in ["command('", "(('", ", ('"] {
        for piece in cli.split(marker).skip(1) {
            let name: String = piece.chars().take_while(char::is_ascii_lowercase).collect();
            if piece[name.len()..].starts_with('\'') && !name.is_empty() {
                commands.insert(name);
            }
        }
    }
    let expected = [
        "clean", "compile", "env", "load", "meta", "model", "run", "suite",
    ];
    assert_eq!(
        commands.iter().map(String::as_str).collect::<Vec<_>>(),
        expected
    );

    let mut used = BTreeSet::new();
    for document in [
        "docs/integration-testing.zh-CN.md",
        "tools/integration-kit/README.md",
    ] {
        let text = read(document);
        for piece in text.split("ppcube-itest ").skip(1) {
            let line = piece.lines().next().unwrap_or_default();
            let line = line.split(['`', '|', '#', '>']).next().unwrap_or_default();
            let mut words = line.split_whitespace();
            let Some(command) = words.next() else {
                continue;
            };
            // Prose after the tool's name is not a command line.
            if !command.chars().all(|c| c.is_ascii_lowercase()) {
                continue;
            }
            if !commands.contains(command) {
                // A word of a sentence, such as "tests" or "在"; a typo of a command would
                // also land here, so it must not look like one.
                assert!(
                    !line.contains("--"),
                    "{document}: `ppcube-itest {line}` is not a command of the kit"
                );
                continue;
            }
            used.insert(command.to_owned());
            for word in words.filter(|word| word.starts_with("--")) {
                let option = word.split('=').next().unwrap();
                assert!(
                    cli.contains(&format!("'{option}'")),
                    "{document}: `{command}` has no option {option}"
                );
            }
        }
    }
    // The guide walks through every command.
    assert_eq!(
        used.iter().map(String::as_str).collect::<Vec<_>>(),
        expected
    );
    // Every key the guide's configuration uses is one the example configuration shows.
    let example = read("tools/integration-kit/itest.example.ini");
    for key in [
        "target",
        "ca",
        "token_file",
        "dialect",
        "schema",
        "table_prefix",
        "data_source",
        "model_version",
        "password_env",
        "replication_num",
        "catalog",
        "url",
    ] {
        assert!(
            example.lines().any(|line| line
                .trim_start_matches("; ")
                .starts_with(&format!("{key} ="))),
            "{key}"
        );
    }
}
