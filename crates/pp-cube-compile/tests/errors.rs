use std::sync::Arc;

use pp_cube_compile::{compile, CompileContext, CompileError, DialectId};
use pp_cube_model::{load_query, load_yaml, Filter};

fn context() -> CompileContext {
    CompileContext {
        request_id: String::new(),
        dialect: DialectId::Presto,
        timezone: "UTC".into(),
        security_filters: vec![],
    }
}

// scenario: B0-RESULT-002
#[test]
fn multi_fact_is_unsupported_and_invalid_inputs_keep_precedence() {
    let base = load_yaml(include_str!("fixtures/retail.yaml")).unwrap();
    let query = load_query(r#"{"measures":["sales.amount","stores.store_count"]}"#).unwrap();
    let error = compile(&Arc::new(base.clone()), &query, &context()).unwrap_err();
    assert!(matches!(error, CompileError::Unsupported(_)), "{error:?}");
    let mut model = base.clone();
    model.cubes[0].sql_table = None;
    assert!(matches!(
        compile(&Arc::new(model), &query, &context()),
        Err(CompileError::Model(_))
    ));
    let mut invalid = query.clone();
    invalid.measures.push("missing.member".into());
    assert!(matches!(
        compile(&Arc::new(base.clone()), &invalid, &context()),
        Err(CompileError::Query(_))
    ));
    let mut ctx = context();
    ctx.security_filters.push(Filter::And(vec![]));
    assert!(matches!(
        compile(&Arc::new(base.clone()), &query, &ctx),
        Err(CompileError::Query(_))
    ));
    let mut model = base;
    model
        .cubes
        .iter_mut()
        .find(|c| c.name == "stores")
        .unwrap()
        .data_source = "other".into();
    assert!(matches!(
        compile(&Arc::new(model), &query, &context()),
        Err(CompileError::Query(_))
    ));
}

// scenario: B0-RESULT-002
#[test]
fn public_failures_do_not_disclose_physical_sources() {
    let text =
        include_str!("fixtures/retail.yaml").replace("dw.sales_dtl", "hidden_schema.hidden_table");
    let base = load_yaml(&text).unwrap();
    for query in [
        r#"{"measures":["missing.member"]}"#,
        r#"{"measures":["sales.amount","stores.store_count"]}"#,
    ] {
        let error = compile(
            &Arc::new(base.clone()),
            &load_query(query).unwrap(),
            &context(),
        )
        .unwrap_err();
        assert!(!error.to_string().contains("hidden_"));
        assert!(error.to_string().contains("missing") || error.to_string().contains("baseline"));
    }
    let mut invalid = base;
    invalid.cubes[0].sql_table = None;
    invalid.cubes[0].sql = Some("SELECT SUM(hidden_column) FROM hidden_schema.hidden_table".into());
    let error = compile(
        &Arc::new(invalid),
        &load_query(r#"{"measures":["sales.amount"]}"#).unwrap(),
        &context(),
    )
    .unwrap_err();
    assert!(!error.to_string().contains("hidden_"));
    let CompileError::Model(issues) = error else {
        panic!("expected Model")
    };
    assert!(!issues.is_empty());
    for issue in issues {
        assert!(!format!("{issue:?}").contains("hidden_"));
    }
}

// scenario: B0-RESULT-002
#[test]
fn security_measure_dependencies_are_classified_without_being_selected() {
    let model = Arc::new(load_yaml(include_str!("fixtures/retail.yaml")).unwrap());
    let query = load_query(r#"{"measures":["sales.amount"]}"#).unwrap();
    let mut ctx = context();
    ctx.security_filters = load_query(
        r#"{"filters":[{"member":"stores.store_count","operator":"gt","values":["1"]}]}"#,
    )
    .unwrap()
    .filters;
    assert!(matches!(
        compile(&model, &query, &ctx),
        Err(CompileError::Unsupported(_))
    ));
    ctx.security_filters =
        load_query(r#"{"filters":[{"member":"missing.member","operator":"set"}]}"#)
            .unwrap()
            .filters;
    assert!(matches!(
        compile(&model, &query, &ctx),
        Err(CompileError::Query(_))
    ));
    for text in [
        r#"{"measures":["sales.amount","refunds.amount"]}"#,
        r#"{"measures":["refunds.amount","stores.store_count"],"dimensions":["regions.region_name"]}"#,
    ] {
        assert!(matches!(
            compile(&model, &load_query(text).unwrap(), &context()),
            Err(CompileError::Query(_))
        ));
    }
}
