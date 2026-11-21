use std::sync::Arc;

use pp_cube_compile::{compile, CompileContext, CompileError, DialectId};
use pp_cube_model::{load_query, load_yaml, validate_model, Filter};

// scenario: B0-VALID-005
#[test]
fn invalid_models_precede_query_and_security_checks_without_mutation() {
    for sql_fault in [false, true] {
        let mut model = load_yaml(include_str!("fixtures/sales.yaml")).unwrap();
        if sql_fault {
            model.cubes[0].sql_table = None;
            model.cubes[0].sql = Some("SELECT SUM(amount) FROM private_sales".into());
        } else {
            model.cubes[0].dimensions[0].sql = "{unknown_member}".into();
        }
        let model = Arc::new(model);
        let expected = validate_model(&model);
        assert!(!expected.is_empty());
        for bad_query in [false, true] {
            for bad_security in [false, true] {
                let query = load_query(if bad_query {
                    r#"{"measures":["missing.member"]}"#
                } else {
                    r#"{"measures":["sales.amount"]}"#
                })
                .unwrap();
                let context = CompileContext {
                    request_id: String::new(),
                    dialect: DialectId::Presto,
                    timezone: "UTC".into(),
                    security_filters: if bad_security {
                        vec![Filter::And(vec![])]
                    } else {
                        vec![]
                    },
                };
                let before = (model.as_ref().clone(), query.clone(), context.clone());
                let error = compile(&model, &query, &context).unwrap_err();
                assert_eq!(error, CompileError::Model(expected.clone()));
                assert_eq!(error.to_string(), "the model could not be validated");
                assert_eq!((model.as_ref().clone(), query, context), before);
            }
        }
    }
}
