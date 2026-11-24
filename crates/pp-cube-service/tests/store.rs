//! Model versions read from shared storage.

use std::path::{Path, PathBuf};

use pp_cube_compile::DialectId;
use pp_cube_service::store::{layout, DirectoryStore, ModelStore, Rejection, StoreSync};
use pp_cube_service::CubeService;

const SALES: &str = "cubes:\n- name: sales\n  sql_table: dw.sales\n  data_source: warehouse\n  joins:\n  - {name: stores, relationship: many_to_one, sql: '{CUBE}.store_cd = {stores.store_cd}'}\n  dimensions:\n  - {name: sale_id, type: number, sql: '{CUBE}.sale_id', primary_key: true}\n  measures:\n  - {name: amount, type: sum, sql: '{CUBE}.amount'}\n";
const STORES: &str = "cubes:\n- name: stores\n  sql_table: dw.stores\n  data_source: warehouse\n  dimensions:\n  - {name: store_cd, type: string, sql: '{CUBE}.store_cd', primary_key: true}\n  - {name: store_name, type: string, sql: '{CUBE}.store_name'}\n";
const VIEW: &str = "views:\n- name: sales_by_store\n  cubes:\n  - {join_path: sales, includes: [amount]}\n  - {join_path: sales.stores, includes: [store_name]}\n";
const MANIFEST: &str = r#"{"schema": 1}"#;

/// A fresh directory under the system's temporary one.
fn root(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("pp-store-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn write(root: &Path, key: &str, text: &str) {
    let path = key
        .split('/')
        .fold(root.to_path_buf(), |path, part| path.join(part));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// Writes a complete version, its manifest last, as a publisher does.
fn publish(root: &Path, at: &str, manifest: &str) {
    write(root, &format!("{at}/cubes/sales.yml"), SALES);
    write(root, &format!("{at}/cubes/stores.yml"), STORES);
    write(root, &format!("{at}/views/sales_by_store.yml"), VIEW);
    write(root, &format!("{at}/manifest.json"), manifest);
}

fn service() -> CubeService {
    CubeService::new([("warehouse".to_owned(), DialectId::Presto)])
}

fn keys(list: &[&str]) -> Vec<String> {
    list.iter().map(|key| (*key).to_owned()).collect()
}

// scenario: B1-STORE-001
#[test]
fn layout_isolates_business_lines_and_asset_types() {
    let found = layout(&keys(&[
        // One, two and three levels of business line.
        "property/models/v1/manifest.json",
        "property/models/v1/cubes/sales.yml",
        "property/motor/models/v2/manifest.json",
        "property/motor/models/v2/cubes/sales.yml",
        "property/motor/fleet/models/2026.11-rc_1/manifest.json",
        "property/motor/fleet/models/2026.11-rc_1/views/v.yml",
        // Still being written: no manifest yet.
        "property/motor/models/v3/cubes/sales.yml",
        // Another asset type, and a file that is nobody's: not this service's.
        "property/motor/metrics/v1/manifest.json",
        "property/README.md",
        "glossary/terms.csv",
    ]));
    let versions: Vec<_> = found.versions.keys().map(String::as_str).collect();
    assert_eq!(
        versions,
        [
            "property/motor/fleet/2026.11-rc_1",
            "property/motor/v2",
            "property/v1"
        ]
    );
    assert_eq!(
        found.versions["property/motor/v2"],
        ["property/motor/models/v2/cubes/sales.yml"]
    );
    assert!(found.rejected.is_empty(), "{:?}", found.rejected);

    // What is under `models` but not in its place is reported, never guessed at.
    let faulty = layout(&keys(&[
        "models/v1/manifest.json",
        "a/b/c/d/models/v1/manifest.json",
        "Property/models/v1/manifest.json",
        "property/models/manifest.json",
        "property/models/v 1/manifest.json",
    ]));
    assert!(faulty.versions.is_empty());
    let reported: Vec<_> = faulty
        .rejected
        .iter()
        .map(|rejected| (rejected.version.as_str(), rejected.reason))
        .collect();
    assert_eq!(
        reported,
        [
            ("Property/models", Rejection::Layout),
            ("a/b/c/d/models", Rejection::Layout),
            ("models", Rejection::Layout),
            ("property/models/manifest.json", Rejection::Layout),
            ("property/v 1", Rejection::Layout),
        ]
    );
}

// scenario: B1-STORE-002
#[test]
fn complete_versions_are_offered_and_a_faulty_one_does_not_stop_the_others() {
    let root = root("offer");
    publish(&root, "property/motor/models/v1", MANIFEST);
    publish(&root, "property/home/models/v1", MANIFEST);
    // Hidden entries are the file system's, not the store's.
    write(
        &root,
        ".snapshot/property/motor/models/v9/manifest.json",
        MANIFEST,
    );
    write(&root, "property/motor/models/v1/.DS_Store", "x");
    let service = service();
    let sync = StoreSync::new(DirectoryStore::new(&root));
    let scan = sync.scan(&service).unwrap();
    assert_eq!(scan.loaded, ["property/home/v1", "property/motor/v1"]);
    assert!(scan.rejected.is_empty(), "{:?}", scan.rejected);
    // The version a caller names is the business line and the version.
    let query = pp_cube_model::load_query(
        r#"{"measures": ["sales_by_store.amount"], "dimensions": ["sales_by_store.store_name"]}"#,
    )
    .unwrap();
    let context = pp_cube_compile::CompileContext {
        request_id: String::new(),
        dialect: DialectId::Presto,
        timezone: "UTC".into(),
        security_filters: vec![],
    };
    assert!(service
        .compile("property/motor/v1", &query, &context)
        .is_ok());
    // Both lines hold the same content, so the same digest; ValidateModel gives it.
    let versions = service.versions();
    assert_eq!(versions[0].1, versions[1].1);
    let digest = versions[0].1.clone();

    // Each fault is its own reason, and none of them is offered.
    publish(
        &root,
        "property/motor/models/v2",
        &format!(r#"{{"schema": 1, "digest": "{digest}"}}"#),
    );
    publish(
        &root,
        "property/motor/models/bad-digest",
        r#"{"schema": 1, "digest": "sha256:00"}"#,
    );
    publish(
        &root,
        "property/motor/models/bad-schema",
        r#"{"schema": 2}"#,
    );
    publish(&root, "property/motor/models/bad-manifest", "published");
    publish(&root, "property/motor/models/bad-file", MANIFEST);
    write(&root, "property/motor/models/bad-file/cubes/notes.txt", "x");
    publish(&root, "property/motor/models/bad-name", MANIFEST);
    write(
        &root,
        "property/motor/models/bad-name/cubes/orders.yml",
        SALES,
    );
    publish(&root, "property/motor/models/bad-model", MANIFEST);
    write(
        &root,
        "property/motor/models/bad-model/cubes/stores.yml",
        &STORES.replace("primary_key: true", "primary_key: false"),
    );
    publish(&root, "property/motor/models/bad-source", MANIFEST);
    write(
        &root,
        "property/motor/models/bad-source/cubes/stores.yml",
        &STORES.replace("data_source: warehouse", "data_source: lake"),
    );
    let scan = sync.scan(&service).unwrap();
    assert_eq!(scan.loaded, ["property/motor/v2"]);
    let rejected: Vec<_> = scan
        .rejected
        .iter()
        .map(|r| (r.version.trim_start_matches("property/motor/"), r.reason))
        .collect();
    assert_eq!(
        rejected,
        [
            ("bad-digest", Rejection::Digest),
            ("bad-file", Rejection::Files),
            ("bad-manifest", Rejection::Manifest),
            ("bad-model", Rejection::Model),
            ("bad-name", Rejection::Files),
            ("bad-schema", Rejection::Manifest),
            ("bad-source", Rejection::DataSource),
        ]
    );
    // The detail names a file or a rule, and no content of the model.
    let detail = |version: &str| {
        let found = scan.rejected.iter().find(|r| r.version.ends_with(version));
        found.unwrap().detail.clone()
    };
    assert!(detail("bad-file").contains("`cubes/notes.txt`"));
    assert!(detail("bad-name").contains("`cubes/orders.yml` defines `sales`"));
    assert!(detail("bad-model").contains("join-primary-key"));
    assert!(scan.rejected.iter().all(|r| !r.detail.contains("dw.")));
    assert_eq!(service.versions().len(), 3);
    // Counted by reason for the metrics, with the passes.
    let metrics = sync.render();
    for line in [
        "ppcube_model_store_scans_total{outcome=\"ok\"} 2",
        "ppcube_model_store_scans_total{outcome=\"error\"} 0",
        "ppcube_model_store_last_scan_failed 0",
        "ppcube_model_store_rejected_versions{reason=\"manifest\"} 2",
        "ppcube_model_store_rejected_versions{reason=\"files\"} 2",
        "ppcube_model_store_rejected_versions{reason=\"digest\"} 1",
    ] {
        assert!(metrics.contains(line), "{line}\n{metrics}");
    }
    // The log names versions and fixed reasons only.
    let lines = pp_cube_service::store::log_lines(&scan);
    assert!(lines.contains(
        &r#"{"event":"model_store","model_version":"property/motor/v2","outcome":"loaded"}"#
            .to_owned()
    ));
    assert!(lines
        .iter()
        .all(|line| !line.contains("cubes/") && !line.contains("dw.")));
    let _ = std::fs::remove_dir_all(root);
}

// scenario: B1-STORE-003
#[test]
fn later_passes_follow_the_store_and_leave_other_versions_alone() {
    let root = root("follow");
    publish(&root, "property/models/v1", MANIFEST);
    let service = service();
    // A version given another way is not the store's to retire.
    service
        .publish("local", pp_cube_model::load_yaml(STORES).unwrap(), None)
        .unwrap();
    let sync = StoreSync::new(DirectoryStore::new(&root));
    assert_eq!(sync.scan(&service).unwrap().loaded, ["property/v1"]);
    // Nothing new: nothing is read again or changed.
    let idle = sync.scan(&service).unwrap();
    assert!(idle.loaded.is_empty() && idle.retired.is_empty() && idle.rejected.is_empty());

    // A version appears when its manifest does, not before.
    write(&root, "property/models/v2/cubes/sales.yml", SALES);
    write(&root, "property/models/v2/cubes/stores.yml", STORES);
    assert!(sync.scan(&service).unwrap().loaded.is_empty());
    write(&root, "property/models/v2/manifest.json", MANIFEST);
    assert_eq!(sync.scan(&service).unwrap().loaded, ["property/v2"]);

    // Versions are immutable: a loaded one is not read again, whatever its files become.
    write(&root, "property/models/v1/cubes/sales.yml", "not a cube");
    assert!(sync.scan(&service).unwrap().rejected.is_empty());
    assert!(service.identity("property/v1").is_ok());

    // Removing the manifest retires it; the version from elsewhere stays.
    std::fs::remove_file(root.join("property/models/v1/manifest.json")).unwrap();
    assert_eq!(sync.scan(&service).unwrap().retired, ["property/v1"]);
    let names: Vec<_> = service
        .versions()
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert_eq!(names, ["local", "property/v2"]);

    // Retired through the service while its manifest stands, it is offered again.
    service.retire("property/v2").unwrap();
    assert_eq!(sync.scan(&service).unwrap().loaded, ["property/v2"]);

    // A store that cannot be listed changes nothing and is counted.
    std::fs::remove_dir_all(&root).unwrap();
    assert!(sync.scan(&service).is_err());
    assert_eq!(service.versions().len(), 2);
    let metrics = sync.render();
    assert!(metrics.contains("ppcube_model_store_scans_total{outcome=\"error\"} 1"));
    assert!(metrics.contains("ppcube_model_store_last_scan_failed 1"));
}

// scenario: B1-STORE-002
#[test]
fn directory_store_reads_only_its_own_keys() {
    let root = root("keys");
    publish(&root, "property/models/v1", MANIFEST);
    let store = DirectoryStore::new(&root);
    assert_eq!(
        store.list().unwrap(),
        [
            "property/models/v1/cubes/sales.yml",
            "property/models/v1/cubes/stores.yml",
            "property/models/v1/manifest.json",
            "property/models/v1/views/sales_by_store.yml",
        ]
    );
    assert_eq!(
        store.read("property/models/v1/manifest.json").unwrap(),
        MANIFEST
    );
    for key in [
        "../outside",
        "property/../../outside",
        "/etc/hosts",
        "property//x",
        "",
    ] {
        assert!(store.read(key).is_err(), "{key}");
    }
    let _ = std::fs::remove_dir_all(root);
}
