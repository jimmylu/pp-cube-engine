//! Joins, segments, and the three reference forms in expressions.

use pp_cube_model::{
    load_yaml, parse_references, Join, Model, Reference, ReferenceError, Relationship, Segment,
    Target,
};

fn model() -> Model {
    load_yaml(include_str!("fixtures/references.yaml")).expect("the fixture loads")
}

// scenario: B0-MODEL-004
#[test]
fn join_and_segment_load_as_declared() {
    let model = model();
    let sales = &model.cubes[0];
    let join = Join {
        name: "dim_store".into(),
        relationship: Relationship::ManyToOne,
        sql: "{CUBE}.store_cd = {dim_store.store_cd}".into(),
    };
    let segment = Segment {
        name: "valid".into(),
        sql: "{CUBE}.is_valid = 1".into(),
    };

    assert_eq!(
        (&sales.joins, &sales.segments),
        (&vec![join], &vec![segment])
    );
}

// scenario: B0-MODEL-004
#[test]
fn join_names_a_declared_cube() {
    let model = model();
    let join = &model.cubes[0].joins[0];

    assert_eq!(model.cube(&join.name), Some(&model.cubes[1]));
}

// scenario: B0-MODEL-004
#[test]
fn join_condition_resolves_to_its_own_column_and_the_other_cube_member() {
    let model = model();
    let (sales, dim_store) = (&model.cubes[0], &model.cubes[1]);

    let targets = model.resolve(sales, &sales.joins[0].sql);

    assert_eq!(
        targets,
        Ok(vec![
            Target::Column {
                cube: sales,
                column: "store_cd"
            },
            Target::Dimension(dim_store, &dim_store.dimensions[0]),
        ])
    );
}

// scenario: B0-MODEL-004
#[test]
fn formula_resolves_to_members_of_its_own_cube() {
    let model = model();
    let sales = &model.cubes[0];

    let ratio = model.resolve(sales, sales.measures[2].sql.as_deref().unwrap_or_default());
    let count = model.resolve(sales, sales.measures[1].sql.as_deref().unwrap_or_default());

    assert_eq!(
        (ratio, count),
        (
            Ok(vec![
                Target::Measure(sales, &sales.measures[0]),
                Target::Measure(sales, &sales.measures[1]),
            ]),
            Ok(vec![Target::Dimension(sales, &sales.dimensions[0])]),
        )
    );
}

// scenario: B0-MODEL-004
#[test]
fn segment_expression_resolves_to_a_column() {
    let model = model();
    let sales = &model.cubes[0];

    let targets = model.resolve(sales, &sales.segments[0].sql);

    assert_eq!(
        targets,
        Ok(vec![Target::Column {
            cube: sales,
            column: "is_valid"
        }])
    );
}

#[test]
fn each_reference_form_is_recognised_in_order() {
    let found = parse_references("{CUBE}.a_1 + {b} - {c.d} + plain_text");

    assert_eq!(
        found,
        Ok(vec![
            Reference::Column("a_1"),
            Reference::OwnMember("b"),
            Reference::Member {
                cube: "c",
                member: "d"
            },
        ])
    );
}

#[test]
fn braces_outside_the_three_forms_are_rejected_where_they_open() {
    let malformed = [
        ("x + {", 4),
        ("{}", 0),
        ("{CUBE}", 0),
        ("{CUBE} + 1", 0),
        ("{CUBE}.1st", 0),
        ("{CUBE.member}", 0),
        ("{a.b.c}", 0),
        ("{ a }", 0),
        ("{a-b}", 0),
        ("{.a}", 0),
        ("{a.}", 0),
        ("{1a}", 0),
        ("{ok} and {not ok}", 9),
    ];

    for (expression, at) in malformed {
        assert_eq!(
            parse_references(expression),
            Err(ReferenceError::Malformed { at }),
            "{expression}"
        );
    }
}

#[test]
fn unknown_cube_is_named_in_the_error() {
    let model = model();

    let error = model.resolve(&model.cubes[0], "{nowhere.store_cd}");

    assert_eq!(
        error,
        Err(ReferenceError::UnknownCube {
            cube: "nowhere".into()
        })
    );
}

#[test]
fn unknown_member_is_named_with_its_cube() {
    let model = model();
    let unknown = |cube: &str, member: &str| {
        Err(ReferenceError::UnknownMember {
            cube: cube.into(),
            member: member.into(),
        })
    };

    let own = model.resolve(&model.cubes[0], "{missing}");
    let other = model.resolve(&model.cubes[0], "{dim_store.missing}");

    assert_eq!(
        (own, other),
        (unknown("sales", "missing"), unknown("dim_store", "missing"))
    );
}

#[test]
fn a_segment_is_not_a_member_an_expression_can_reference() {
    let model = model();

    let error = model.resolve(&model.cubes[0], "{valid}");

    assert_eq!(
        error,
        Err(ReferenceError::UnknownMember {
            cube: "sales".into(),
            member: "valid".into()
        })
    );
}
