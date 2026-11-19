//! What does not load, and where the error points.

use pp_cube_model::{load_json, load_yaml, Position};
use serde_json::json;

fn yaml_with_meta(value: &str) -> String {
    format!("cubes:\n  - name: sales\n    data_source: warehouse\n    meta:\n      n: {value}\n")
}

fn json_with_meta(value: &str) -> String {
    format!(
        r#"{{"cubes": [{{"name": "sales", "data_source": "warehouse", "meta": {{"n": {value}}}}}]}}"#
    )
}

#[test]
fn unknown_yaml_field_is_rejected_at_its_position() {
    let text = "cubes:\n  - name: sales\n    data_source: warehouse\n    pre_aggregations: []\n";

    let error = load_yaml(text).expect_err("a field outside the baseline does not load");

    assert_eq!(error.position, Some(Position { line: 4, column: 5 }));
    assert!(
        error.message.contains("pre_aggregations"),
        "{}",
        error.message
    );
}

#[test]
fn unknown_yaml_type_value_is_rejected_at_its_position() {
    let text = concat!(
        "cubes:\n  - name: sales\n    data_source: warehouse\n    dimensions:\n",
        "      - name: place\n        sql: \"{CUBE}.place\"\n        type: geo\n",
    );

    let error = load_yaml(text).expect_err("a type outside the baseline does not load");

    assert_eq!(
        error.position,
        Some(Position {
            line: 7,
            column: 15
        })
    );
    assert!(error.message.contains("geo"), "{}", error.message);
}

#[test]
fn unknown_json_field_is_rejected_on_its_line() {
    let text = "{\"cubes\": [{\n  \"name\": \"sales\",\n  \"data_source\": \"warehouse\",\n  \"views\": []\n}]}";

    let error = load_json(text).expect_err("a field outside the baseline does not load");

    assert_eq!(error.position.map(|position| position.line), Some(4));
    assert!(error.message.contains("views"), "{}", error.message);
}

#[test]
fn json_that_is_not_strict_json_is_rejected() {
    for text in [
        "{\"cubes\": [],}",
        "{\"cubes\": [] /* none */}",
        "{\"cubes\": []} x",
    ] {
        let loaded = load_json(text);

        assert!(loaded.is_err(), "{text} loaded as {loaded:?}");
    }
}

#[test]
fn measure_without_sql_is_rejected_unless_its_type_is_count() {
    let text = concat!(
        "cubes:\n  - name: sales\n    data_source: warehouse\n    measures:\n",
        "      - name: row_count\n        type: count\n",
        "      - name: amount\n        type: sum\n",
    );

    let error = load_yaml(text).expect_err("only a count measure may omit sql");

    assert!(error.message.contains("`amount`"), "{}", error.message);
}

#[test]
fn meta_integer_outside_64_bits_is_rejected_from_both_formats() {
    let too_large = "18446744073709551617";
    let too_small = "-9223372036854775809";
    let loaded = [
        load_yaml(&yaml_with_meta(too_large)),
        load_yaml(&yaml_with_meta(too_small)),
        load_json(&json_with_meta(too_large)),
        load_json(&json_with_meta(too_small)),
    ];

    for result in loaded {
        let error = result.expect_err("the integer cannot be kept exactly");
        assert!(
            error.message.contains("cannot be kept exactly"),
            "{}",
            error.message
        );
    }
}

#[test]
fn meta_number_that_is_not_finite_is_rejected() {
    for value in [".nan", ".inf", "-.inf"] {
        let loaded = load_yaml(&yaml_with_meta(value));

        assert!(loaded.is_err(), "{value} loaded as {loaded:?}");
    }
}

#[test]
fn widest_64_bit_integers_in_meta_are_kept_exactly() {
    let value = "[18446744073709551615, -9223372036854775808, 9007199254740993]";
    let expected = Some(json!({"n": [u64::MAX, i64::MIN, 9_007_199_254_740_993_u64]}));

    let from_yaml = load_yaml(&yaml_with_meta(value))
        .expect("YAML loads")
        .cubes
        .remove(0)
        .meta;
    let from_json = load_json(&json_with_meta(value))
        .expect("JSON loads")
        .cubes
        .remove(0)
        .meta;

    assert_eq!((from_yaml, from_json), (expected.clone(), expected));
}
