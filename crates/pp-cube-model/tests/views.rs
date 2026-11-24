//! Views in the model contract.

use pp_cube_model::{
    load_json, load_yaml, parse_references, validate_yaml, AllMembers, Includes, MeasureType,
    Reference,
};

const MODEL: &str = r#"
cubes:
  - name: sales
    sql_table: dw.sales_dtl
    data_source: warehouse
    joins:
      - { name: stores, relationship: many_to_one, sql: "{CUBE}.store_cd = {stores.store_cd}" }
    dimensions:
      - { name: sale_id, sql: "{CUBE}.sale_id", type: number, primary_key: true }
      - { name: channel_cd, sql: "{CUBE}.channel_cd", type: string }
      - name: sold_at
        sql: "{CUBE}.sold_at"
        type: time
        granularities:
          - { name: fiscal_year, interval: 1 year, offset: 3 month }
    measures:
      - { name: amount, type: sum, sql: "{CUBE}.amount" }
      - { name: amount_7d, type: sum, sql: "{CUBE}.amount", rolling_window: { trailing: 7 day } }
    segments:
      - { name: toys, sql: "{CUBE}.category_cd = 'TOY'" }
  - name: stores
    sql_table: dw.dim_store
    data_source: warehouse
    joins:
      - { name: regions, relationship: many_to_one, sql: "{CUBE}.region_cd = {regions.region_cd}" }
    dimensions:
      - { name: store_cd, sql: "{CUBE}.store_cd", type: string, primary_key: true }
      - { name: store_name, sql: "{CUBE}.store_name", type: string }
    measures:
      - { name: amount, type: sum, sql: "{CUBE}.floor_area" }
  - name: regions
    sql_table: dw.dim_region
    data_source: warehouse
    dimensions:
      - { name: region_cd, sql: "{CUBE}.region_cd", type: string, primary_key: true }
      - { name: region_name, sql: "{CUBE}.region_name", type: string }
views:
  - name: sales_view
    title: Sales
    cubes:
      - { join_path: sales, includes: [amount, amount_7d, channel_cd, sold_at] }
      - { join_path: sales.stores, includes: "*", excludes: [store_cd], prefix: true }
      - { join_path: sales.stores.regions, includes: [region_name] }
"#;

// scenario: B1-MODEL-003
#[test]
fn views_load_as_written_and_expand_to_cubes_of_references() {
    let model = load_yaml(MODEL).unwrap();
    let view = &model.views[0];
    assert_eq!(
        (view.name.as_str(), view.title.as_deref()),
        ("sales_view", Some("Sales"))
    );
    assert_eq!(view.cubes[1].includes, Includes::All(AllMembers::All));
    assert_eq!(view.cubes[1].excludes, ["store_cd"]);
    assert_eq!(view.cubes[1].prefix, Some(true));
    assert_eq!(view.cubes[2].path(), ["sales", "stores", "regions"]);
    let json = serde_json::to_string(&model).unwrap();
    assert_eq!(load_json(&json).unwrap(), model);
    assert!(
        validate_yaml(MODEL).is_empty(),
        "{:?}",
        validate_yaml(MODEL)
    );
    // A model without views is written without the field.
    let mut plain = model.clone();
    plain.views.clear();
    assert!(serde_json::to_value(&plain).unwrap().get("views").is_none());

    // Expanded, the view is one more cube: each member refers to the member it
    // takes by its join path, a measure as a formula over it.
    let expanded = model.expanded();
    assert_eq!(expanded.cubes.len(), 4);
    assert_eq!(expanded.expanded(), expanded);
    assert!(expanded.is_view("sales_view") && !expanded.is_view("sales"));
    let cube = expanded.cube("sales_view").unwrap();
    let dimensions: Vec<_> = cube
        .dimensions
        .iter()
        .map(|d| (d.name.as_str(), d.sql.as_str()))
        .collect();
    assert_eq!(
        dimensions,
        [
            ("channel_cd", "{sales.channel_cd}"),
            ("sold_at", "{sales.sold_at}"),
            ("stores_store_name", "{sales.stores.store_name}"),
            ("region_name", "{sales.stores.regions.region_name}"),
        ]
    );
    let measures: Vec<_> = cube
        .measures
        .iter()
        .map(|m| (m.name.as_str(), m.measure_type, m.sql.as_deref()))
        .collect();
    assert_eq!(
        measures,
        [
            ("amount", MeasureType::Number, Some("{sales.amount}")),
            ("amount_7d", MeasureType::Number, Some("{sales.amount_7d}")),
            (
                "stores_amount",
                MeasureType::Number,
                Some("{sales.stores.amount}")
            ),
        ]
    );
    // The window stays with the measure it is taken from; the periods a time
    // dimension declares come along; no member of a view is a key.
    assert!(cube.measures[1].rolling_window.is_none());
    assert_eq!(cube.dimensions[1].granularities.len(), 1);
    assert!(cube.dimensions.iter().all(|d| !d.primary_key));
    assert_eq!(cube.data_source, "warehouse");
    // What a view member stands for.
    assert_eq!(
        expanded.underlying("sales_view.stores_amount"),
        "stores.amount"
    );
    assert_eq!(
        expanded.underlying("sales_view.region_name"),
        "regions.region_name"
    );
    assert_eq!(expanded.underlying("sales.amount"), "sales.amount");
    // The reference form views are made of.
    assert_eq!(
        parse_references("{sales.stores.amount}"),
        Ok(vec![Reference::Joined {
            path: "sales.stores",
            member: "amount"
        }])
    );
}

fn issues(original: &str, replacement: &str) -> Vec<(String, String)> {
    assert!(MODEL.contains(original), "{original}");
    let issues = validate_yaml(&MODEL.replace(original, replacement));
    let mut pairs: Vec<_> = issues.into_iter().map(|i| (i.path, i.rule_id)).collect();
    // One fault may break two rules of the same kind at one field.
    pairs.dedup();
    pairs
}

fn issue(path: &str, rule: &str) -> Vec<(String, String)> {
    vec![(path.to_owned(), rule.to_owned())]
}

// scenario: B1-VALID-004
#[test]
fn view_rules_are_reported_at_their_fields() {
    let first = "{ join_path: sales, includes: [amount, amount_7d, channel_cd, sold_at] }";
    let second =
        r#"{ join_path: sales.stores, includes: "*", excludes: [store_cd], prefix: true }"#;
    let third = "{ join_path: sales.stores.regions, includes: [region_name] }";
    for (original, replacement, path, rule) in [
        // Shape.
        (
            first,
            "{ join_path: sales }",
            "/views/0/cubes/0/includes",
            "required",
        ),
        (
            first,
            "{ join_path: sales, includes: all }",
            "/views/0/cubes/0/includes",
            "type",
        ),
        (
            first,
            "{ join_path: sales, includes: [amount], alias: s }",
            "/views/0/cubes/0/alias",
            "unknown-field",
        ),
        (
            second,
            "{ join_path: sales.stores, includes: [store_name], prefix: yes please }",
            "/views/0/cubes/1/prefix",
            "type",
        ),
        (
            "    title: Sales",
            "    title: Sales\n    measures: []",
            "/views/0/measures",
            "unknown-field",
        ),
        // A path is a chain of joins the model declares, from the view's one root.
        (
            third,
            "{ join_path: sales.regions, includes: [region_name] }",
            "/views/0/cubes/2/join_path",
            "view",
        ),
        (
            third,
            "{ join_path: stores.sales, includes: [amount] }",
            "/views/0/cubes/2/join_path",
            "view",
        ),
        (
            third,
            "{ join_path: missing, includes: [region_name] }",
            "/views/0/cubes/2/join_path",
            "view",
        ),
        // Members are dimensions and measures of the cube the path ends at.
        (
            first,
            "{ join_path: sales, includes: [amount, missing] }",
            "/views/0/cubes/0/includes/1",
            "view",
        ),
        (
            first,
            "{ join_path: sales, includes: [amount, toys] }",
            "/views/0/cubes/0/includes/1",
            "view",
        ),
        (
            second,
            r#"{ join_path: sales.stores, includes: "*", excludes: [missing], prefix: true }"#,
            "/views/0/cubes/1/excludes/0",
            "view",
        ),
        // One name, one member: both cubes have an `amount`.
        (
            second,
            r#"{ join_path: sales.stores, includes: "*", excludes: [store_cd] }"#,
            "/views/0/cubes/1",
            "duplicate-name",
        ),
        // A view is not a cube's namesake.
        (
            "  - name: sales_view",
            "  - name: stores",
            "/views/0/name",
            "duplicate-name",
        ),
    ] {
        assert_eq!(
            issues(original, replacement),
            issue(path, rule),
            "{replacement}"
        );
    }
    // A stores path that starts elsewhere than the view's root is reported twice:
    // it is not a declared chain from there, and it is a second root.
    assert_eq!(
        issues(first, "{ join_path: regions, includes: [region_name] }"),
        [
            ("/views/0/cubes/1/join_path".to_owned(), "view".to_owned()),
            ("/views/0/cubes/2".to_owned(), "duplicate-name".to_owned()),
            ("/views/0/cubes/2/join_path".to_owned(), "view".to_owned()),
        ]
    );
    // A model expression does not write a join path itself.
    assert_eq!(
        issues(
            r#"sql: "{CUBE}.floor_area""#,
            r#"sql: "{stores.regions.region_cd}""#
        ),
        issue("/cubes/1/measures/0/sql", "reference")
    );
}
