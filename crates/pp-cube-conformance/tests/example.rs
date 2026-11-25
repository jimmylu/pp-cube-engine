//! The runnable example under `examples/insurance-analytics` asks what the
//! property-insurance scenario asks. Its model, questions and expected answers
//! are copies a reader can run; this keeps the copies true without a JVM, a
//! service or a warehouse.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use pp_cube_compile::{compile, CompileContext, CompileError, DialectId};
use pp_cube_model::{assemble, load_query, load_yaml, ModelFile};
use serde_json::Value;

fn example() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/insurance-analytics")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The files of the one version in the example's store, as the service reads them.
fn published() -> Vec<(String, String)> {
    let version = example().join("model-store/property/models/2026-11");
    assert!(read(&version.join("manifest.json")).contains("\"schema\": 1"));
    let mut files = Vec::new();
    for folder in ["cubes", "views"] {
        let mut entries: Vec<_> = std::fs::read_dir(version.join(folder))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        entries.sort();
        for path in entries {
            let name = path.file_name().unwrap().to_str().unwrap();
            files.push((format!("{folder}/{name}"), read(&path)));
        }
    }
    files
}

// scenario: B1-CLIENT-001
#[test]
fn the_example_asks_the_insurance_scenario_and_expects_its_answers() {
    // The published model is the scenario's model, one cube or view a file.
    let files = published();
    let files: Vec<ModelFile<'_>> = files
        .iter()
        .map(|(path, text)| ModelFile { path, text })
        .collect();
    let model = assemble(&files).expect("the example's store holds a model");
    // A store lists its files by name; the scenario's file has its own order.
    let mut scenario = load_yaml(include_str!("../fixtures/insurance.yaml")).unwrap();
    scenario.cubes.sort_by(|a, b| a.name.cmp(&b.name));
    scenario.views.sort_by(|a, b| a.name.cmp(&b.name));
    let mut model = model;
    model.cubes.sort_by(|a, b| a.name.cmp(&b.name));
    model.views.sort_by(|a, b| a.name.cmp(&b.name));
    assert!(
        model == scenario,
        "the example's model is not the scenario's"
    );
    let model = Arc::new(model);

    let portal: Value = serde_json::from_str(&read(&example().join("portal.json"))).unwrap();
    assert_eq!(portal["modelVersion"], "property/2026-11");
    assert_eq!(portal["dialect"], "presto");
    let manifest: Value =
        serde_json::from_str(&pp_cube_conformance::manifest_with_sources("presto").unwrap())
            .unwrap();
    let case = |name: &str| {
        let cases = manifest["cases"].as_array().unwrap();
        cases.iter().find(|case| case["name"] == name).cloned()
    };

    let (mut answered, mut refused, mut restricted) = (0, 0, 0);
    for user in portal["users"].as_array().unwrap() {
        let rules = user["rowFilters"].as_array().unwrap();
        let security_filters = load_query(&serde_json::json!({ "filters": rules }).to_string())
            .unwrap()
            .filters;
        for question in user["questions"].as_array().unwrap() {
            let name = question["name"].as_str().unwrap();
            let who = format!("{}/{name}", user["name"].as_str().unwrap());
            let query = load_query(&question["query"].to_string())
                .unwrap_or_else(|error| panic!("{who}: {error}"));
            let context = CompileContext {
                request_id: String::new(),
                dialect: DialectId::Presto,
                timezone: portal["timezone"].as_str().unwrap().into(),
                security_filters: security_filters.clone(),
            };
            let compiled = compile(&model, &query, &context);
            if let Some(known) = question.get("refused") {
                // What the example shows a caller doing with a refusal, by its kind.
                let kind = match compiled {
                    Err(CompileError::Unsupported(_)) => "UNSUPPORTED",
                    Err(CompileError::Query(_)) => "QUERY",
                    other => panic!("{who}: {other:?}"),
                };
                assert_eq!(known["kind"], kind, "{who}");
                assert!(question.get("expect").is_none(), "{who}");
                refused += 1;
                continue;
            }
            let compiled = compiled.unwrap_or_else(|error| panic!("{who}: {error}"));
            let expect = &question["expect"];
            let columns: Vec<&str> = compiled.columns.iter().map(|c| c.alias.as_str()).collect();
            assert_eq!(expect["columns"], serde_json::json!(columns), "{who}");
            // The same question as the executed case of that name, and its rows:
            // those were worked out by hand and run on the engines.
            let executed = if rules.is_empty() {
                case(name)
            } else {
                restricted += 1;
                None
            };
            if let Some(executed) = executed {
                assert_eq!(question["query"], executed["query"], "{who}");
                assert_eq!(expect["rows"], executed["rows"], "{who}");
                assert!(executed["security_filters"].as_array().unwrap().is_empty());
            }
            answered += 1;
        }
    }
    // A user with a rule asks the question of the security case, under its rule.
    let east = &portal["users"][3];
    let secured = case("security_region_across_facts").unwrap();
    assert_eq!(east["rowFilters"], secured["security_filters"]);
    assert_eq!(east["questions"][0]["query"], secured["query"]);
    assert_eq!(east["questions"][0]["expect"]["rows"], secured["rows"]);
    assert_eq!((answered, refused, restricted), (15, 2, 2));
}

// scenario: B1-CLIENT-002
#[test]
fn the_example_runs_the_service_as_the_shipped_deployment_does() {
    // The example is a deployment in small: its service takes the arguments of
    // deploy/kubernetes.yaml, but for how often it reads its store.
    let arguments = |text: &str, key: &str| -> Vec<String> {
        text.lines()
            .skip_while(|line| line.trim() != key)
            .skip(1)
            .map_while(|line| line.trim().strip_prefix("- \""))
            .map(|rest| rest.strip_suffix('"').unwrap().to_owned())
            .collect()
    };
    let compose = arguments(&read(&example().join("docker-compose.yaml")), "command:");
    let shipped = arguments(
        &read(&example().join("../../deploy/kubernetes.yaml")),
        "args:",
    );
    assert!(compose.len() > 10 && shipped.len() > compose.len());
    for pair in compose.chunks(2) {
        let (flag, value) = (&pair[0], &pair[1]);
        let at = shipped.iter().position(|a| a == flag).expect(flag);
        if flag == "--model-refresh-seconds" {
            continue;
        }
        assert_eq!(&shipped[at + 1], value, "{flag}");
    }
    // The Java client is built from the service's contract, not from a copy.
    let pom = read(&example().join("../../clients/java/pom.xml"));
    assert!(pom.contains(
        "<protoSourceRoot>${project.basedir}/../../crates/pp-cube-service/proto</protoSourceRoot>"
    ));
    let version = format!("<version>{}</version>", env!("CARGO_PKG_VERSION"));
    assert!(
        pom.contains(&version),
        "the client carries the kernel's version"
    );
    assert!(read(&example().join("app/pom.xml")).contains(&version));
    // A Java file lives where its package says, under the contract's own name.
    // javac does not insist on it, so a package named after something else
    // builds and runs; this is what notices.
    let mut sources = Vec::new();
    for root in ["../../clients/java/src", "app/src"] {
        collect(&example().join(root), &mut sources);
    }
    assert!(sources.len() >= 6, "{sources:?}");
    for path in &sources {
        let text = read(path);
        let declared = text
            .lines()
            .find_map(|line| line.strip_prefix("package "))
            .and_then(|rest| rest.strip_suffix(';'))
            .unwrap_or_else(|| panic!("{}: no package", path.display()));
        assert!(declared.starts_with("dev.ppcube."), "{}", path.display());
        let folder = path.parent().unwrap().to_str().unwrap().replace('\\', "/");
        let placed = format!("/java/{}", declared.replace('.', "/"));
        assert!(
            folder.ends_with(&placed),
            "{}: package {declared}",
            path.display()
        );
    }
    let contract = read(&example().join("../../crates/pp-cube-service/proto/engine.proto"));
    assert!(contract.contains("package ppcube.engine.v1;"));
    assert!(contract.contains("option java_package = \"dev.ppcube.engine.v1\";"));
}

/// Every `.java` file under `folder`.
fn collect(folder: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name != "target") {
                collect(&path, found);
            }
        } else if path
            .extension()
            .is_some_and(|extension| extension == "java")
        {
            found.push(path);
        }
    }
}
