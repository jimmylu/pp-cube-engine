//! Loading a model from YAML and from JSON.

use pp_cube_model::{
    load_json, load_yaml, Dimension, DimensionType as D, LoadError, Measure, MeasureFilter,
    MeasureType as M, Model,
};
use serde_json::json;

const YAML: &str = include_str!("fixtures/baseline.yaml");
const JSON: &str = include_str!("fixtures/baseline.json");

fn yaml_model() -> Model {
    load_yaml(YAML).expect("the YAML fixture loads")
}

// scenario: B0-MODEL-001
#[test]
fn yaml_and_json_load_the_same_model() {
    assert_eq!(Ok(yaml_model()), load_json(JSON));
}

// scenario: B0-MODEL-001
#[test]
fn every_dimension_type_is_loaded() {
    let model = yaml_model();

    let types: Vec<_> = model.cubes[0]
        .dimensions
        .iter()
        .map(|d| d.dimension_type)
        .collect();

    assert_eq!(types, [D::Number, D::String, D::Boolean, D::Time]);
}

// scenario: B0-MODEL-001
#[test]
fn every_measure_type_is_loaded() {
    let model = yaml_model();

    let types: Vec<_> = model.cubes[0]
        .measures
        .iter()
        .map(|m| m.measure_type)
        .collect();

    assert_eq!(
        types,
        [
            M::Count,
            M::CountDistinct,
            M::Sum,
            M::Avg,
            M::Min,
            M::Max,
            M::Number
        ]
    );
}

// scenario: B0-MODEL-001
#[test]
fn optional_dimension_fields_and_meta_are_loaded_as_written() {
    let expected = Dimension {
        name: "order_no".into(),
        sql: "{CUBE}.order_no".into(),
        dimension_type: D::Number,
        primary_key: true,
        title: Some("Order number".into()),
        description: Some("Order identifier".into()),
        public: Some(false),
        // Neither float is one a parser that rounds reads exactly.
        meta: Some(json!({
            "owner": "team-a", "tier": 2, "ratio": 1.2699041298821259, "tolerance": 1.5e-30,
            "certified": true, "retired_on": null, "tags": ["finance", "daily"],
            "contact": {"channel": "analytics"}
        })),
    };

    assert_eq!(yaml_model().cubes[0].dimensions[0], expected);
}

// scenario: B0-MODEL-001
#[test]
fn optional_measure_fields_are_loaded_as_written() {
    let expected = Measure {
        name: "amount".into(),
        measure_type: M::Sum,
        sql: Some("{CUBE}.amount".into()),
        filters: vec![MeasureFilter {
            sql: "{CUBE}.category_cd = 'TOY'".into(),
        }],
        title: Some("Amount".into()),
        description: Some("Net sales amount".into()),
        public: Some(true),
        meta: Some(json!({"unit": "each"})),
        format: Some("currency".into()),
    };

    assert_eq!(yaml_model().cubes[0].measures[2], expected);
}

#[test]
fn model_survives_a_json_round_trip() {
    let model = yaml_model();

    let text = serde_json::to_string(&model).expect("a model serialises");

    assert_eq!(load_json(&text), Ok(model));
}

#[test]
fn contract_types_can_be_shared_across_threads() {
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<Model>();
    assert_send_sync::<LoadError>();
}
