use std::sync::Arc;

use pp_cube_compile::{compile, CompileContext, DialectId};
use pp_cube_model::{load_yaml, ModelBundle, ModelCatalog, Query};

// scenario: B0-MODEL-003
#[test]
fn alternating_bundle_versions_compile_without_cross_version_mutation() {
    let source = include_str!("fixtures/sales.yaml");
    let mut catalog = ModelCatalog::default();
    let first = catalog
        .register(ModelBundle::new("v1", "caller-digest-a", load_yaml(source).unwrap()).unwrap())
        .unwrap();
    let changed = source.replace("dw.sales_dtl", "dw.sales_v2");
    let second = catalog
        .register(ModelBundle::new("v2", "caller-digest-b", load_yaml(&changed).unwrap()).unwrap())
        .unwrap();
    let originals = [
        first.model().as_ref().clone(),
        second.model().as_ref().clone(),
    ];
    let query = Query {
        measures: vec!["sales.amount".into()],
        ..Query::default()
    };
    let context = CompileContext {
        request_id: "bundle-test".into(),
        dialect: DialectId::Presto,
        timezone: "UTC".into(),
        security_filters: vec![],
    };
    let a = compile(first.model(), &query, &context).unwrap();
    let b = compile(second.model(), &query, &context).unwrap();
    assert!(a.sql.contains("dw.sales_dtl"));
    assert!(b.sql.contains("dw.sales_v2"));
    assert_ne!(a.fingerprint, b.fingerprint);
    for _ in 0..3 {
        for (version, expected) in [("v2", &b), ("v1", &a)] {
            let selected = catalog.get(version).unwrap();
            assert_eq!(
                &compile(selected.model(), &query, &context).unwrap(),
                expected
            );
        }
    }
    drop(catalog);
    assert_eq!(first.model().as_ref(), &originals[0]);
    assert_eq!(second.model().as_ref(), &originals[1]);
    // A caller can mutate a detached copy without affecting an active bundle.
    let mut detached = Arc::clone(first.model());
    Arc::make_mut(&mut detached).cubes.clear();
    assert_eq!(compile(first.model(), &query, &context).unwrap(), a);
}
