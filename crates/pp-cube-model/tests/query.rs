//! The query type, as far as the kernel compiles queries.

use pp_cube_model::{load_query, Query};

#[test]
fn query_loads_from_cube_query_json() {
    let text = r#"{"measures": ["sales.amount"], "dimensions": ["sales.channel_cd"]}"#;
    let expected = Query {
        measures: vec!["sales.amount".into()],
        dimensions: vec!["sales.channel_cd".into()],
    };

    assert_eq!(load_query(text), Ok(expected));
}

#[test]
fn query_field_the_kernel_does_not_compile_yet_is_rejected() {
    let text = r#"{"measures": ["sales.amount"], "filters": []}"#;

    let error = load_query(text).expect_err("filters are not compiled yet");

    assert!(error.message.contains("filters"), "{}", error.message);
}
