use pp_cube_compile::{CompileContext, DialectId};
use pp_cube_model::{load_yaml, Query};
use pp_cube_service::{CompilePool, CubeService};
use std::sync::Arc;

// scenario: B0-API-005
#[test]
fn async_results_preserve_versions_errors_and_security_context() {
    let service = CubeService::new([("warehouse".to_owned(), DialectId::Presto)]);
    for version in ["v1", "v2"] {
        let model = load_yaml(&format!("cubes:\n- name: sales\n  sql_table: dw.{version}\n  data_source: warehouse\n  dimensions:\n  - {{name: region, type: string, sql: '{{CUBE}}.region'}}\n  measures:\n  - {{name: amount, type: sum, sql: '{{CUBE}}.amount'}}\n")).unwrap();
        service.publish(version, model, None).unwrap();
    }
    let service = Arc::new(service);
    let pool = CompilePool::new(Arc::clone(&service), 2, 8).unwrap();
    let query = Query {
        measures: vec!["sales.amount".into()],
        ..Query::default()
    };
    let context = CompileContext {
        request_id: "worker-test".into(),
        dialect: DialectId::Presto,
        timezone: "Asia/Tokyo".into(),
        security_filters: pp_cube_model::load_query(
            r#"{"filters":[{"member":"sales.region","operator":"equals","values":["north"]}]}"#,
        )
        .unwrap()
        .filters,
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    for version in ["v1", "v2", "missing"] {
        let task = pool
            .submit(version.into(), query.clone(), context.clone())
            .unwrap();
        let actual = runtime.block_on(task.wait());
        assert_eq!(
            actual,
            service
                .compile(version, &query, &context)
                .map_err(pp_cube_service::PoolError::Compile)
        );
    }
    pool.shutdown();
}
