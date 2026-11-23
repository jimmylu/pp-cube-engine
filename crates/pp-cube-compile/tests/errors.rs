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
// scenario: B1-MULTI-005
#[test]
fn invalid_inputs_keep_precedence_in_a_query_over_two_cubes_measures() {
    let base = load_yaml(include_str!("fixtures/retail.yaml")).unwrap();
    let query = load_query(r#"{"measures":["sales.amount","stores.store_count"]}"#).unwrap();
    assert!(compile(&Arc::new(base.clone()), &query, &context()).is_ok());
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

// scenario: B0-QUERY-009
// scenario: B0-AGG-003
#[test]
fn recognized_out_of_b0_features_are_unsupported_after_model_validation() {
    let model = load_yaml(include_str!("fixtures/retail.yaml")).unwrap();
    let boundary_queries = [
        r#"{"timeDimensions":[{"dimension":"sales.sold_at","compareDateRange":[["2025-10-01","2025-10-31"]]}]}"#,
        r#"{"timeDimensions":[{"dimension":"sales.sold_at","granularity":"second"}]}"#,
        r#"{"timeDimensions":[{"dimension":"sales.sold_at","granularity":"minute"}]}"#,
        r#"{"timeDimensions":[{"dimension":"sales.sold_at","granularity":"hour"}]}"#,
    ];
    for text in boundary_queries {
        let query = load_query(text).expect("recognized feature loads");
        let error = compile(&Arc::new(model.clone()), &query, &context()).unwrap_err();
        assert!(
            matches!(error, CompileError::Unsupported(_)),
            "{text}: {error:?}"
        );
    }

    let mut invalid = model.clone();
    invalid.cubes[0].sql_table = None;
    for text in boundary_queries {
        let query = load_query(text).unwrap();
        assert!(
            matches!(
                compile(&Arc::new(invalid.clone()), &query, &context()),
                Err(CompileError::Model(_))
            ),
            "{text}"
        );
    }

    let grouped = load_query(r#"{"ungrouped":false,"measures":["sales.amount"]}"#).unwrap();
    assert!(compile(&Arc::new(model), &grouped, &context()).is_ok());
}

// scenario: B0-RESULT-002
#[test]
fn public_failures_do_not_disclose_physical_sources() {
    let text =
        include_str!("fixtures/retail.yaml").replace("dw.sales_dtl", "hidden_schema.hidden_table");
    let base = load_yaml(&text).unwrap();
    for query in [
        r#"{"measures":["missing.member"]}"#,
        r#"{"measures":["sales.amount","refunds.amount"],"dimensions":["sales.channel_cd"]}"#,
    ] {
        let error = compile(
            &Arc::new(base.clone()),
            &load_query(query).unwrap(),
            &context(),
        )
        .unwrap_err();
        assert!(!error.to_string().contains("hidden_"));
        assert!(
            error.to_string().contains("missing") || error.to_string().contains("No join path")
        );
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
        r#"{"measures":["sales.amount","refunds.amount"],"dimensions":["sales.channel_cd"]}"#,
        r#"{"measures":["refunds.amount","stores.store_count"],"dimensions":["regions.region_name"]}"#,
    ] {
        assert!(matches!(
            compile(&model, &load_query(text).unwrap(), &context()),
            Err(CompileError::Query(_))
        ));
    }
}

// scenario: B0-RESULT-002
#[test]
fn upstream_depth_failure_names_filter_member_and_actual_rule() {
    // Debug planner frames are large; reach its explicit 160-level guard on a
    // bounded test thread stack instead of exhausting libtest's default stack.
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(assert_depth_error)
        .unwrap()
        .join()
        .unwrap();
}

fn assert_depth_error() {
    let mut text = String::from("cubes:\n  - name: sales\n    sql_table: hidden_schema.hidden_table\n    data_source: warehouse\n    measures:\n");
    for i in 0..170 {
        text.push_str(&format!(
            "      - {{name: m{i}, type: number, sql: \"{{m{}}}\"}}\n",
            i + 1
        ));
    }
    text.push_str("      - {name: m170, type: sum, sql: \"{CUBE}.hidden_column\"}\n");
    let model = Arc::new(load_yaml(&text).unwrap());
    let query = load_query(r#"{"measures":["sales.m170"],"filters":[{"member":"sales.m0","operator":"gt","values":["1"]}]}"#).unwrap();
    let error = compile(&model, &query, &context()).unwrap_err();
    let CompileError::Query(message) = error else {
        panic!("expected Query: {error:?}")
    };
    assert!(message.contains("sales.m0"), "{message}");
    assert!(message.contains("resolution depth limit"), "{message}");
    assert!(!message.contains("hidden_"));
}
