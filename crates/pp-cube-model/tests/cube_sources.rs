use pp_cube_model::{load_json, load_yaml, validate_json, validate_yaml};
use serde_json::{json, Value};

fn reports(model: &Value) -> [Vec<(String, String)>; 2] {
    let issues = [
        validate_json(&model.to_string()),
        validate_yaml(&serde_norway::to_string(model).unwrap()),
    ];
    issues.map(|report| report.into_iter().map(|i| (i.path, i.rule_id)).collect())
}

// Serialization may add absent defaults, but every supplied field must survive.
fn assert_preserved(input: &Value, written: &Value) {
    match input {
        Value::Object(fields) => {
            for (name, value) in fields {
                assert_preserved(value, &written[name]);
            }
        }
        Value::Array(values) => {
            let actual = written.as_array().expect("the collection survives");
            assert_eq!(values.len(), actual.len());
            for (value, actual) in values.iter().zip(actual) {
                assert_preserved(value, actual);
            }
        }
        _ => assert_eq!(input, written),
    }
}

// scenario: B0-MODEL-002
#[test]
fn every_source_selection_failure_is_located_in_both_formats() {
    let base = json!({"cubes":[{
        "name":"sales", "data_source":"warehouse", "sql_table":"sales"
    }]});
    for (missing, extra_sql, expected) in [
        (Some("name"), false, ("/cubes/0/name", "required")),
        (
            Some("data_source"),
            false,
            ("/cubes/0/data_source", "required"),
        ),
        (Some("sql_table"), false, ("/cubes/0", "source")),
        (None, true, ("/cubes/0", "source")),
    ] {
        let mut model = base.clone();
        let cube = model["cubes"][0].as_object_mut().unwrap();
        if let Some(field) = missing {
            cube.remove(field);
        }
        if extra_sql {
            cube.insert("sql".into(), json!("SELECT id FROM sales"));
        }
        let expected = vec![(expected.0.to_owned(), expected.1.to_owned())];
        assert_eq!(reports(&model), [expected.clone(), expected], "{model}");
    }
}

// scenario: B0-MODEL-006
#[test]
fn all_cube_fields_survive_each_source_and_unknown_fields_are_rejected() {
    for (source, expression) in [
        ("sql_table", "sales"),
        ("sql", "SELECT id FROM sales WHERE active = 1"),
    ] {
        let mut cube = json!({
            "name":"sales", "data_source":"warehouse", "sql_alias":"sales_rows",
            "title":"Sales", "description":"Sales rows", "public":false,
            "meta":{"team":"analytics", "revision":2},
            "joins":[{"name":"stores", "relationship":"many_to_one",
                      "sql":"{CUBE}.store_id = {stores.id}"}],
            "dimensions":[{"name":"id", "sql":"{CUBE}.id",
                           "type":"number", "primary_key":true}],
            "measures":[{"name":"amount", "type":"sum", "sql":"{CUBE}.amount"}],
            "segments":[{"name":"active", "sql":"{CUBE}.active = 1"}]
        });
        cube[source] = json!(expression);
        let model = json!({"cubes":[cube, {
            "name":"stores", "data_source":"warehouse", "sql_table":"stores",
            "dimensions":[{"name":"id", "sql":"{CUBE}.id",
                           "type":"number", "primary_key":true}]
        }]});
        assert_eq!(reports(&model), [vec![], vec![]]);
        let from_json = load_json(&model.to_string()).unwrap();
        let from_yaml = load_yaml(&serde_norway::to_string(&model).unwrap()).unwrap();
        assert_eq!(from_json, from_yaml);
        let written = serde_json::to_value(&from_json).unwrap();
        // The absent alternative source is serialized as null.
        let other = if source == "sql" { "sql_table" } else { "sql" };
        assert!(written["cubes"][0][other].is_null());
        assert_preserved(&model, &written);
        assert_eq!(
            load_json(&serde_json::to_string(&from_yaml).unwrap()).unwrap(),
            from_json
        );
        assert_eq!(
            load_yaml(&serde_norway::to_string(&from_json).unwrap()).unwrap(),
            from_json
        );
        let mut unknown = model;
        unknown["cubes"][0]["extra_field"] = json!(true);
        let expected = vec![("/cubes/0/extra_field".into(), "unknown-field".into())];
        assert_eq!(reports(&unknown), [expected.clone(), expected]);
        assert!(load_json(&unknown.to_string()).is_err());
        assert!(load_yaml(&serde_norway::to_string(&unknown).unwrap()).is_err());
    }
}
