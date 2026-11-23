use pp_cube_model::{load_yaml, BundleError, Model, ModelBundle, ModelCatalog};
use std::sync::Arc;

fn model(table: &str) -> Model {
    load_yaml(&format!("cubes:\n- name: sales\n  data_source: warehouse\n  sql_table: dw.{table}\n  measures:\n  - {{name: amount, type: sum, sql: '{{CUBE}}.amount'}}\n")).unwrap()
}

// scenario: B0-MODEL-007
#[test]
fn invalid_input_preserves_registered_version() {
    let mut catalog = ModelCatalog::default();
    let first = catalog
        .register(ModelBundle::new("v1", "digest-a", model("first")).unwrap())
        .unwrap();
    for (version, digest) in [(" ", "digest"), ("v2", "\t")] {
        assert!(matches!(
            ModelBundle::new(version, digest, model("second")),
            Err(BundleError::InvalidIdentifier { .. })
        ));
    }
    let mut invalid = model("second");
    invalid.cubes[0].measures[0].sql = None;
    let expected_issues = pp_cube_model::validate_model(&invalid);
    let Err(BundleError::Model(issues)) = ModelBundle::new("v2", "digest-b", invalid) else {
        panic!("expected model issues")
    };
    assert!(!issues.is_empty());
    assert_eq!(issues, expected_issues);
    assert!(issues.iter().all(|issue| !issue.path.is_empty()));
    assert!(matches!(
        catalog.get("v2"),
        Err(BundleError::UnknownVersion)
    ));
    assert!(Arc::ptr_eq(&first, &catalog.get("v1").unwrap()));
}

// scenario: B0-MODEL-008
#[test]
fn exact_selection_duplicate_rejection_and_handle_lifetime() {
    let mut catalog = ModelCatalog::default();
    let original = model("first");
    let first = catalog
        .register(ModelBundle::new("v1", "digest-a", original.clone()).unwrap())
        .unwrap();
    for digest in ["digest-a", "digest-b"] {
        assert!(matches!(
            catalog.register(ModelBundle::new("v1", digest, model("second")).unwrap()),
            Err(BundleError::DuplicateVersion)
        ));
    }
    assert!(matches!(
        catalog.get("missing"),
        Err(BundleError::UnknownVersion)
    ));
    assert!(matches!(
        catalog.get(" v1"),
        Err(BundleError::UnknownVersion)
    ));
    let second = catalog
        .register(ModelBundle::new(" v2 ", " digest-b ", model("second")).unwrap())
        .unwrap();
    drop(catalog);
    assert_eq!(first.version(), "v1");
    assert_eq!(first.digest(), "digest-a");
    assert_eq!(first.model().as_ref(), &original);
    assert_eq!(second.version(), " v2 ");
    assert_eq!(second.digest(), " digest-b ");
    let mut detached = Arc::clone(first.model());
    Arc::make_mut(&mut detached).cubes.clear();
    assert_eq!(first.model().as_ref(), &original);
}

// scenario: B0-MODEL-009
#[test]
fn a_catalog_lists_versions_in_order_and_retires_without_revoking_handles() {
    let mut catalog = ModelCatalog::default();
    for version in ["v2", "v10", "v1"] {
        catalog
            .register(ModelBundle::new(version, "digest", model(version)).unwrap())
            .unwrap();
    }
    let versions = |catalog: &ModelCatalog| {
        catalog
            .bundles()
            .iter()
            .map(|bundle| bundle.version().to_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(versions(&catalog), ["v1", "v10", "v2"]);
    let held = catalog.get("v1").unwrap();
    let retired = catalog.retire("v1").unwrap();
    assert!(std::sync::Arc::ptr_eq(&held, &retired));
    // The handle keeps its content; the catalog no longer offers the version.
    assert_eq!(held.model().cubes.len(), 1);
    assert_eq!(catalog.get("v1").unwrap_err(), BundleError::UnknownVersion);
    assert_eq!(
        catalog.retire("v1").unwrap_err(),
        BundleError::UnknownVersion
    );
    assert_eq!(versions(&catalog), ["v10", "v2"]);
    // A retired version can be registered again.
    catalog
        .register(ModelBundle::new("v1", "other", model("again")).unwrap())
        .unwrap();
    assert_eq!(catalog.get("v1").unwrap().digest(), "other");
}
