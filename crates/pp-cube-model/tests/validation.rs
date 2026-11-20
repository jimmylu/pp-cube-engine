use pp_cube_model::{validate_json, validate_yaml};

// scenario: B0-VALID-001
#[test]
fn syntax_is_located_at_root_and_valid_models_have_no_issues() {
    assert_eq!(validate_json("{")[0].path, "");
    assert_eq!(validate_yaml("[")[0].rule_id, "syntax");
    assert!(validate_yaml(include_str!("fixtures/baseline.yaml")).is_empty());
    assert!(validate_json(include_str!("fixtures/baseline.json")).is_empty());
}

// scenario: B0-VALID-001
#[test]
fn malformed_reference_does_not_hide_independent_references() {
    let json = JOIN.replace(
        "{CUBE}.amount",
        "{missing_before} + {bad.reference.extra} + {missing_after}",
    );
    assert_eq!(
        validate_json(&json)
            .iter()
            .filter(|i| i.rule_id == "reference")
            .count(),
        3
    );
}

fn pairs(issues: Vec<pp_cube_model::ValidationIssue>) -> Vec<(String, String)> {
    issues.into_iter().map(|i| (i.path, i.rule_id)).collect()
}
// scenario: B0-VALID-001
#[test]
fn independent_field_faults_are_collected_with_escaped_pointers() {
    let json = r#"{"cubes":[{"name":"sales","data_source":3,"sql_table":"sales",
      "bad/key~":true,"dimensions":[{"name":"id","type":"geo","extra":1}],
      "measures":[{"name":"amount","type":"sum"}]}]}"#;
    let expected = [
        ("/cubes/0/bad~1key~0", "unknown-field"),
        ("/cubes/0/data_source", "type"),
        ("/cubes/0/dimensions/0/extra", "unknown-field"),
        ("/cubes/0/dimensions/0/sql", "required"),
        ("/cubes/0/dimensions/0/type", "type"),
        ("/cubes/0/measures/0/sql", "required"),
    ]
    .map(|(p, r)| (p.to_owned(), r.to_owned()));
    assert_eq!(pairs(validate_json(json)), expected);
    assert!(pp_cube_model::load_json(json).is_err());
}
// scenario: B0-VALID-001
#[test]
fn metadata_precision_and_strict_json_are_preserved() {
    for meta in ["9223372036854775808.0", "18446744073709551616"] {
        let json = format!(
            r#"{{"cubes":[{{"name":"sales","data_source":"main","sql_table":"sales","meta":{meta}}}]}}"#
        );
        assert_eq!(
            pairs(validate_json(&json)),
            [("/cubes/0/meta".into(), "type".into())]
        );
        assert!(pp_cube_model::load_json(&json).is_err());
    }
    for json in [r#"{"cubes":[],}"#, r#"{/*comment*/"cubes":[]}"#] {
        assert_eq!(validate_json(json)[0].rule_id, "syntax");
    }
    for meta in ["18446744073709551615", "-9223372036854775808", "1.25"] {
        let json = format!(
            r#"{{"cubes":[{{"name":"sales","data_source":"main","sql_table":"sales","meta":{meta}}}]}}"#
        );
        assert!(validate_json(&json).is_empty());
    }
    let yaml = "cubes:\n- name: sales\n  sql_table: sales\n  data_source: main\n  meta: .nan";
    assert_eq!(validate_yaml(yaml)[0].path, "/cubes/0/meta");
}
const JOIN: &str = r#"{"cubes":[
 {"name":"sales","sql_table":"sales","data_source":"main",
 "joins":[{"name":"stores","relationship":"many_to_one","sql":"{CUBE}.store_id = {stores.id}"}],
 "dimensions":[{"name":"id","sql":"{CUBE}.id","type":"number","primary_key":true}],
 "measures":[{"name":"amount","sql":"{CUBE}.amount","type":"sum"}]},
 {"name":"stores","sql_table":"stores","data_source":"main",
 "dimensions":[{"name":"id","sql":"{CUBE}.id","type":"number","primary_key":true}]}]}"#;
// scenario: B0-VALID-001
#[test]
fn formats_and_typed_models_share_semantics() {
    let model = pp_cube_model::load_json(JOIN).unwrap();
    assert!(pp_cube_model::validate_model(&model).is_empty());
    let mut value: serde_json::Value = serde_json::from_str(JOIN).unwrap();
    value["cubes"][0]["sql"] = "SELECT * FROM sales".into();
    value["cubes"][0]["measures"][0]
        .as_object_mut()
        .unwrap()
        .remove("sql");
    let json = serde_json::to_string(&value).unwrap();
    let yaml = serde_norway::to_string(&value).unwrap();
    assert_eq!(validate_json(&json), validate_yaml(&yaml));
    let mut typed = model;
    typed.cubes[0].sql = Some("SELECT * FROM sales".into());
    typed.cubes[0].measures[0].sql = None;
    assert_eq!(validate_json(&json), pp_cube_model::validate_model(&typed));
    assert_eq!(
        pairs(validate_json(&json)),
        [
            ("/cubes/0".into(), "source".into()),
            ("/cubes/0/measures/0/sql".into(), "required".into())
        ]
    );
}
// scenario: B0-VALID-001
#[test]
fn references_and_shape_faults_do_not_short_circuit_other_definitions() {
    let json = JOIN
        .replace(
            "{CUBE}.amount",
            "{missing_a} + {stores.missing_b} + {absent.member}",
        )
        .replace("\"data_source\":\"main\"", "\"data_source\":false");
    let issues = validate_json(&json);
    assert_eq!(
        issues.iter().filter(|i| i.rule_id == "reference").count(),
        3
    );
    assert_eq!(issues.iter().filter(|i| i.rule_id == "type").count(), 2);
    assert_eq!(issues, validate_json(&json));
    let malformed = JOIN.replace("{CUBE}.amount", "{broken");
    assert_eq!(validate_json(&malformed)[0].rule_id, "reference");
}
// scenario: B0-VALID-001
#[test]
fn each_join_endpoint_needs_a_key_and_pairs_are_unique() {
    for endpoint in [0, 1] {
        let mut v: serde_json::Value = serde_json::from_str(JOIN).unwrap();
        v["cubes"][endpoint]["dimensions"][0]["primary_key"] = false.into();
        let issues = validate_json(&v.to_string());
        assert_eq!(
            pairs(issues),
            [("/cubes/0/joins/0".into(), "join-primary-key".into())]
        );
    }
    let mut v: serde_json::Value = serde_json::from_str(JOIN).unwrap();
    let join = v["cubes"][0]["joins"][0].clone();
    v["cubes"][0]["joins"].as_array_mut().unwrap().push(join);
    v["cubes"][1]["joins"] = serde_json::json!([{"name":"sales","relationship":"one_to_one","sql":"{CUBE}.id = {sales.id}"}]);
    assert_eq!(
        pairs(validate_json(&v.to_string())),
        [
            ("/cubes/0/joins/1".into(), "duplicate-join".into()),
            ("/cubes/1/joins/0".into(), "duplicate-join".into())
        ]
    );
    v["cubes"][0]["joins"][0]["name"] = "absent".into();
    assert!(validate_json(&v.to_string())
        .iter()
        .any(|i| i.rule_id == "join-target"));
}
// scenario: B0-VALID-001
#[test]
fn duplicate_names_and_unusable_target_lists_do_not_create_cascades() {
    let mut v: serde_json::Value = serde_json::from_str(JOIN).unwrap();
    v["cubes"][1]["dimensions"] = false.into();
    assert_eq!(
        pairs(validate_json(&v.to_string())),
        [("/cubes/1/dimensions".into(), "type".into())]
    );
    let mut model = pp_cube_model::load_json(JOIN).unwrap();
    let dimension = model.cubes[0].dimensions[0].clone();
    model.cubes[0].dimensions.push(dimension);
    model.cubes.push(model.cubes[0].clone());
    assert!(pp_cube_model::validate_model(&model)
        .iter()
        .any(|i| i.rule_id == "duplicate-name"));
}
