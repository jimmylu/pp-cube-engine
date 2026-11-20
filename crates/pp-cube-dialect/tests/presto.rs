//! The Presto dialect: its templates render, and a missing one is named.

use minijinja::context;
use pp_cube_dialect::{dialect, DialectError, DialectId};

#[test]
fn known_template_renders_with_its_context() {
    let presto = dialect(DialectId::Presto).expect("the Presto data loads");

    let rendered = presto.render(
        "expressions/query_aliased",
        context! { query => "dw.sales_dtl", quoted_alias => "\"sales\"" },
    );

    assert_eq!(rendered, Ok("dw.sales_dtl AS \"sales\"".to_owned()));
}

#[test]
fn unknown_template_fails_naming_it() {
    let presto = dialect(DialectId::Presto).expect("the Presto data loads");

    let rendered = presto.render("statements/no_such_template", context! {});

    assert_eq!(
        rendered,
        Err(DialectError::UnknownTemplate {
            dialect: "presto",
            template: "statements/no_such_template".into(),
        })
    );
}

#[test]
fn presto_quotes_with_double_quotes_and_numbers_no_parameters() {
    let presto = dialect(DialectId::Presto).expect("the Presto data loads");

    let quote = presto.template("quotes/identifiers").map(String::as_str);
    let parameter = presto.template("params/param").map(String::as_str);

    assert_eq!(
        (quote, parameter, presto.reuses_parameters()),
        (Some("\""), Some("?"), false)
    );
}
