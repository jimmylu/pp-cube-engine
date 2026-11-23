use pp_cube_compile::{compile, CompileContext, DialectId};
use pp_cube_model::{load_query, load_yaml};
use std::sync::Arc;
// scenario: B0-RESULT-001
#[test]
fn result_corresponds_to_actual_output() {
    let model = Arc::new(load_yaml(include_str!("fixtures/retail.yaml")).unwrap());
    let query=load_query(r#"{"measures":["sales.amount"],"dimensions":["regions.region_name"],"filters":[{"member":"regions.region_name","operator":"equals","values":["A","B"]}]}"#).unwrap();
    let ctx = CompileContext {
        dialect: DialectId::Presto,
        timezone: "UTC".into(),
        security_filters: vec![],
        request_id: "one".into(),
    };
    let result = compile(&model, &query, &ctx).unwrap();
    assert_eq!(
        result
            .columns
            .iter()
            .map(|c| c.member.as_str())
            .collect::<Vec<_>>(),
        ["regions.region_name", "sales.amount"]
    );
    assert_eq!(
        result
            .used_cubes
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>(),
        ["regions", "sales", "stores"]
    );
    assert_eq!(
        result.parameters,
        [
            pp_cube_compile::Parameter::Text("A".into()),
            pp_cube_compile::Parameter::Text("B".into())
        ]
    );
    assert_eq!(
        result
            .columns
            .iter()
            .map(|c| (c.alias.as_str(), c.semantic_type))
            .collect::<Vec<_>>(),
        [
            ("regions__region_name", pp_cube_model::DimensionType::String),
            ("sales__amount", pp_cube_model::DimensionType::Number)
        ]
    );
}

fn context() -> CompileContext {
    CompileContext {
        dialect: DialectId::Presto,
        timezone: "UTC".into(),
        security_filters: vec![],
        request_id: String::new(),
    }
}
// scenario: B0-RESULT-001
#[test]
fn lineage_and_time_columns_follow_rendered_sources() {
    let mut model = load_yaml(include_str!("fixtures/retail.yaml")).unwrap();
    let query=load_query(r#"{"measures":["sales.amount"],"dimensions":["regions.region_name"],"timeDimensions":[{"dimension":"sales.sold_at","granularity":"month"}]}"#).unwrap();
    let result = compile(&Arc::new(model.clone()), &query, &context()).unwrap();
    assert_eq!(
        result
            .used_cubes
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>(),
        ["regions", "sales", "stores"]
    );
    assert!(result
        .columns
        .iter()
        .any(|c| c.member == "sales.sold_at" && c.granularity.as_deref() == Some("month")));
    let sales = model.cubes.iter_mut().find(|c| c.name == "sales").unwrap();
    sales.sql_table = None;
    sales.sql = Some("SELECT * FROM \"warehouse\".\"Sales\"".into());
    sales.sql_alias = Some("custom_sales".into());
    let result = compile(&Arc::new(model), &query, &context()).unwrap();
    let sales = result
        .used_cubes
        .iter()
        .find(|c| c.name == "sales")
        .unwrap();
    assert_eq!(
        sales.physical_tables,
        [vec!["warehouse".to_owned(), "Sales".to_owned()]]
    );
}
// scenario: B0-DET-002
#[test]
fn fingerprint_covers_model_query_context_but_not_request_id() {
    let model = load_yaml(include_str!("fixtures/sales.yaml")).unwrap();
    let query = load_query(r#"{"measures":["sales.amount"]}"#).unwrap();
    let ctx = context();
    let base = compile(&Arc::new(model.clone()), &query, &ctx).unwrap();
    let mut correlation = ctx.clone();
    correlation.request_id = "another".into();
    assert_eq!(
        base,
        compile(&Arc::new(model.clone()), &query, &correlation).unwrap()
    );
    let mut changed = model.clone();
    changed.cubes[0].title = Some("changed".into());
    assert_ne!(
        base.fingerprint,
        compile(&Arc::new(changed), &query, &ctx)
            .unwrap()
            .fingerprint
    );
    let mut changed = query.clone();
    changed.limit = Some(7);
    assert_ne!(
        base.fingerprint,
        compile(&Arc::new(model.clone()), &changed, &ctx)
            .unwrap()
            .fingerprint
    );
    let mut changed = ctx.clone();
    changed.timezone = "Asia/Tokyo".into();
    assert_ne!(
        base.fingerprint,
        compile(&Arc::new(model.clone()), &query, &changed)
            .unwrap()
            .fingerprint
    );
    let mut changed = ctx.clone();
    changed.security_filters = load_query(
        r#"{"filters":[{"member":"sales.channel_cd","operator":"equals","values":["A"]}]}"#,
    )
    .unwrap()
    .filters;
    assert_ne!(
        base.fingerprint,
        compile(&Arc::new(model), &query, &changed)
            .unwrap()
            .fingerprint
    );
}

// scenario: B0-QUERY-011
// scenario: B0-RESULT-001
#[test]
fn explicit_grouped_false_preserves_sql_and_absent_fields_stay_absent() {
    let model = Arc::new(load_yaml(include_str!("fixtures/sales.yaml")).unwrap());
    let absent = load_query(r#"{"measures":["sales.amount"]}"#).unwrap();
    let explicit = load_query(r#"{"measures":["sales.amount"],"ungrouped":false}"#).unwrap();
    let absent_json = serde_json::to_string(&absent).unwrap();
    assert!(!absent_json.contains("ungrouped"));
    assert!(!absent_json.contains("compareDateRange"));
    let first = compile(&model, &absent, &context()).unwrap();
    let second = compile(&model, &explicit, &context()).unwrap();
    assert_eq!(first.sql, second.sql);
    assert_eq!(first.parameters, second.parameters);
}
// scenario: B0-DET-002
#[test]
fn fingerprint_child() {
    if std::env::var_os("PP_FINGERPRINT_CHILD").is_none() {
        return;
    }
    let m = Arc::new(load_yaml(include_str!("fixtures/sales.yaml")).unwrap());
    let q = load_query(r#"{"measures":["sales.amount"]}"#).unwrap();
    println!(
        "IDENTITY:{:?}",
        compile(&m, &q, &context()).unwrap().fingerprint
    );
}
// scenario: B0-DET-002
#[test]
fn fingerprint_is_stable_across_fresh_processes() {
    let run = || {
        let out = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "fingerprint_child", "--nocapture"])
            .env("PP_FINGERPRINT_CHILD", "1")
            .output()
            .unwrap();
        assert!(out.status.success());
        String::from_utf8(out.stdout)
            .unwrap()
            .lines()
            .find(|l| l.starts_with("IDENTITY:"))
            .unwrap()
            .to_owned()
    };
    assert_eq!(run(), run());
}
// scenario: B0-RESULT-001
#[test]
fn hidden_filters_repetitions_and_aliases_do_not_create_columns() {
    let model = Arc::new(load_yaml(include_str!("fixtures/retail.yaml")).unwrap());
    let query=load_query(r#"{"measures":["sales.amount","sales.amount"],"timeDimensions":[{"dimension":"sales.sold_at","dateRange":["2025-01-01","2025-01-31"]}]}"#).unwrap();
    let mut ctx = context();
    ctx.security_filters = load_query(
        r#"{"filters":[{"member":"stores.store_name","operator":"equals","values":["North"]}]}"#,
    )
    .unwrap()
    .filters;
    let result = compile(&model, &query, &ctx).unwrap();
    // The pinned planner preserves both selected occurrences; hidden filters add none.
    assert_eq!(result.columns.len(), 2);
    assert!(result.columns.iter().all(|c| c.member == "sales.amount"));
    assert!(result.used_cubes.iter().any(|c| c.name == "stores"));
    assert!(result.columns.iter().all(|c| c.alias == "sales__amount"));
}
// scenario: B0-RESULT-001
#[test]
fn out_of_baseline_cte_is_not_reported_as_a_physical_table() {
    let mut model = load_yaml(include_str!("fixtures/sales.yaml")).unwrap();
    model.cubes[0].sql_table = None;
    model.cubes[0].sql = Some("WITH x AS (SELECT * FROM dw.sales_dtl) SELECT * FROM x".into());
    let query = load_query(r#"{"measures":["sales.amount"]}"#).unwrap();
    let model = Arc::new(model);
    assert!(matches!(
        compile(&model, &query, &context()),
        Err(pp_cube_compile::CompileError::Model(_))
    ));
    // Direct adapter callers get an error too, never lineage containing alias x.
    let dialect = pp_cube_dialect::dialect(DialectId::Presto).unwrap();
    assert!(pp_cube_adapter::plan(&model, &query, dialect, "UTC").is_err());
}

// scenario: B0-RESULT-003
// scenario: B0-PRESTO-017
#[test]
fn session_properties_follow_the_dialect_and_the_offset() {
    let model = Arc::new(load_yaml(include_str!("fixtures/sales.yaml")).unwrap());
    let plain = load_query(r#"{"measures":["sales.amount"],"limit":5}"#).unwrap();
    let paged = load_query(r#"{"measures":["sales.amount"],"limit":5,"offset":10}"#).unwrap();
    let properties = |dialect, query| {
        let ctx = CompileContext {
            dialect,
            ..context()
        };
        let result = compile(&model, query, &ctx).unwrap();
        // The property is declared exactly when the SQL uses the clause that needs it.
        assert_eq!(
            result.sql.contains("OFFSET"),
            query.offset.is_some(),
            "{dialect:?}"
        );
        result
            .session_properties
            .into_iter()
            .map(|p| (p.name, p.value))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        properties(DialectId::Presto, &paged),
        [("offset_clause_enabled".to_owned(), "true".to_owned())]
    );
    assert!(properties(DialectId::Presto, &plain).is_empty());
    assert!(properties(DialectId::Doris, &paged).is_empty());
    assert!(properties(DialectId::Doris, &plain).is_empty());
}
