//! A model expression in the form the planner takes.

use std::any::Any;
use std::fmt::Write as _;
use std::rc::Rc;

use cubenativeutils::CubeError;
use cubesqlplanner::cube_bridge::member_sql::{
    CompiledMemberTemplate, MemberSql, SqlTemplate, SqlTemplateArgs,
};
use pp_cube_model::locate_references;

/// The expression text with `{arg:N}` in place of each reference, and the
/// path that each N stands for. The planner resolves a path against the cube
/// the expression belongs to: `CUBE` is that cube, one name is one of its
/// members, two names are a member of another cube.
pub(crate) struct Expression {
    template: String,
    args: SqlTemplateArgs,
    arg_names: Vec<String>,
}

impl Expression {
    pub(crate) fn compile(text: &str) -> Result<Rc<Self>, CubeError> {
        let located = locate_references(text)
            .map_err(|error| CubeError::user(format!("{error}, in `{text}`")))?;
        let mut template = String::with_capacity(text.len());
        let mut args = SqlTemplateArgs::default();
        let mut arg_names = Vec::new();
        let mut copied = 0;
        for found in located {
            let names = found.reference.names();
            let path: Vec<String> = names.into_iter().map(str::to_owned).collect();
            if !arg_names.contains(&path[0]) {
                arg_names.push(path[0].clone());
            }
            let index = args.insert_symbol_path(path);
            template.push_str(&text[copied..found.braces.start]);
            // Writing to a String cannot fail.
            let _ = write!(template, "{{arg:{index}}}");
            copied = found.braces.end;
        }
        template.push_str(&text[copied..]);
        Ok(Rc::new(Self {
            template,
            args,
            arg_names,
        }))
    }

    pub(crate) fn compiled(&self) -> CompiledMemberTemplate {
        CompiledMemberTemplate {
            template: SqlTemplate::String(self.template.clone()),
            args: self.args.clone(),
        }
    }
}

impl MemberSql for Expression {
    fn args_names(&self) -> &Vec<String> {
        &self.arg_names
    }

    fn as_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compiled(text: &str) -> (String, Vec<Vec<String>>, Vec<String>) {
        let expression = Expression::compile(text).expect("the expression compiles");
        let SqlTemplate::String(template) = expression.compiled().template else {
            panic!("a single template is expected");
        };
        (
            template,
            expression.args.symbol_paths.clone(),
            expression.arg_names.clone(),
        )
    }

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn column_keeps_its_name_and_refers_to_the_owning_cube() {
        let expected = (
            "{arg:0}.amount".to_owned(),
            vec![strings(&["CUBE"])],
            strings(&["CUBE"]),
        );

        assert_eq!(compiled("{CUBE}.amount"), expected);
    }

    #[test]
    fn members_become_paths_and_a_repeated_reference_shares_its_argument() {
        let expected = (
            "{arg:0} / NULLIF({arg:1}, 0) + {arg:0} + {arg:2}".to_owned(),
            vec![
                strings(&["amount"]),
                strings(&["order_count"]),
                strings(&["dim_store", "store_cd"]),
            ],
            strings(&["amount", "order_count", "dim_store"]),
        );

        assert_eq!(
            compiled("{amount} / NULLIF({order_count}, 0) + {amount} + {dim_store.store_cd}"),
            expected
        );
    }

    #[test]
    fn text_without_references_is_unchanged() {
        assert_eq!(
            compiled("dw.sales_dtl"),
            ("dw.sales_dtl".to_owned(), vec![], vec![])
        );
    }

    #[test]
    fn malformed_braces_are_a_user_error_quoting_the_expression() {
        let error = Expression::compile("{CUBE}")
            .err()
            .expect("bare {CUBE} is malformed");

        assert!(error.message.contains("`{CUBE}`"), "{}", error.message);
    }
}
