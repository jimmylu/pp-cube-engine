use pp_cube_compile::{compile, CompileContext, DialectId};
use pp_cube_model::{load_query, load_yaml};
use std::sync::Arc;
// scenario: B0-RESULT-001
#[test]
fn result_corresponds_to_actual_output() {
    let model = Arc::new(load_yaml(include_str!("fixtures/sales.yaml")).unwrap());
    let query=load_query(r#"{"measures":["sales.amount"],"dimensions":["sales.channel_cd"],"filters":[{"member":"sales.channel_cd","operator":"equals","values":["A","B"]}]}"#).unwrap();
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
        ["sales.channel_cd", "sales.amount"]
    );
    assert_eq!(result.used_cubes[0].name, "sales");
    assert_eq!(result.parameters.len(), 2);
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
    assert!(result.columns.iter().all(|c| c.member == "sales.amount"));
    assert!(result.used_cubes.iter().any(|c| c.name == "stores"));
    assert!(result.columns.iter().all(|c| c.alias == "sales__amount"));
}
