//! A model as a directory of files, one cube or view each.

use pp_cube_model::{assemble, load_yaml, validate_model, AssembleError, ModelFile};

const SALES: &str = r#"
cubes:
  - name: sales
    sql_table: dw.sales_dtl
    data_source: warehouse
    joins:
      - { name: stores, relationship: many_to_one, sql: "{CUBE}.store_cd = {stores.store_cd}" }
    dimensions:
      - { name: sale_id, sql: "{CUBE}.sale_id", type: number, primary_key: true }
    measures:
      - { name: amount, type: sum, sql: "{CUBE}.amount" }
"#;
const STORES: &str = r#"
cubes:
  - name: stores
    sql_table: dw.dim_store
    data_source: warehouse
    dimensions:
      - { name: store_cd, sql: "{CUBE}.store_cd", type: string, primary_key: true }
      - { name: store_name, sql: "{CUBE}.store_name", type: string }
"#;
const VIEW: &str = r#"
views:
  - name: sales_by_store
    cubes:
      - { join_path: sales, includes: [amount] }
      - { join_path: sales.stores, includes: [store_name] }
"#;

fn file<'a>(path: &'a str, text: &'a str) -> ModelFile<'a> {
    ModelFile { path, text }
}

// scenario: B1-MODEL-004
#[test]
fn directory_of_one_cube_or_view_per_file_is_one_model_whatever_the_order() {
    let files = [
        file("views/sales_by_store.yml", VIEW),
        file("cubes/stores.yaml", STORES),
        file("cubes/sales.yml", SALES),
    ];
    let model = assemble(&files).unwrap();
    let names: Vec<_> = model.cubes.iter().map(|cube| cube.name.as_str()).collect();
    assert_eq!(names, ["sales", "stores"]);
    assert_eq!(model.views[0].name, "sales_by_store");
    assert!(validate_model(&model).is_empty());
    // The same files in another order are the same model.
    let mut reversed = files;
    reversed.reverse();
    assert_eq!(assemble(&reversed).unwrap(), model);
    // And it is the model one document with the same content would be.
    let single = format!(
        "{}{}{}",
        SALES.trim_end(),
        STORES.replace("cubes:\n", ""),
        VIEW
    );
    assert_eq!(load_yaml(&single).unwrap(), model);
}

// scenario: B1-MODEL-004
#[test]
fn file_that_is_misplaced_misnamed_or_holds_more_than_one_is_named_in_the_error() {
    let refused = |files: &[ModelFile<'_>]| assemble(files).unwrap_err();
    for path in [
        "sales.yml",
        "cubes/sales.json",
        "cubes/Sales.yml",
        "cubes/sales-detail.yml",
        "cubes/retail/sales.yml",
        "segments/sales.yml",
        "cubes/.yml",
    ] {
        assert_eq!(
            refused(&[file(path, SALES)]),
            AssembleError::UnexpectedFile { path: path.into() },
            "{path}"
        );
    }
    assert_eq!(
        refused(&[file("cubes/orders.yml", SALES)]),
        AssembleError::Misnamed {
            path: "cubes/orders.yml".into(),
            found: "sales".into()
        }
    );
    let two = format!("{}{}", SALES.trim_end(), STORES.replace("cubes:\n", ""));
    for (path, text, kind) in [
        ("cubes/sales.yml", two.as_str(), "cube"),
        ("cubes/sales.yml", "cubes: []", "cube"),
        ("cubes/sales_by_store.yml", VIEW, "cube"),
        ("views/sales.yml", SALES, "view"),
    ] {
        assert_eq!(
            refused(&[file(path, text)]),
            AssembleError::NotOne {
                path: path.into(),
                kind
            },
            "{path}"
        );
    }
    // A fault in a file is located in that file.
    let broken = SALES.replace("type: sum", "type: total");
    let error = refused(&[
        file("cubes/stores.yml", STORES),
        file("cubes/sales.yml", &broken),
    ]);
    let AssembleError::Syntax { path, position, .. } = &error else {
        panic!("expected Syntax, got {error:?}")
    };
    assert_eq!(
        (path.as_str(), position.map(|at| at.line)),
        ("cubes/sales.yml", Some(11))
    );
    assert!(error.to_string().starts_with("`cubes/sales.yml`: "));
    // An unknown top-level key is not ignored; views alone are not a model.
    assert!(matches!(
        refused(&[file("cubes/sales.yml", &format!("{SALES}\nsegments: []"))]),
        AssembleError::Syntax { .. }
    ));
    assert_eq!(
        refused(&[file("views/sales_by_store.yml", VIEW)]),
        AssembleError::NoCubes
    );
    assert_eq!(refused(&[]), AssembleError::NoCubes);
}
