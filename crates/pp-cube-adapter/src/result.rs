//! Metadata paired with the final rendered projection, never query-list order.
use crate::PlanError;
use cubesqlplanner::planner::MemberSymbol;
use pp_cube_model::{DimensionType, Model};
use sqlparser::{
    ast::{SelectItem, SetExpr, Statement, TableFactor, Visit, Visitor},
    dialect::GenericDialect,
    parser::Parser,
};
use std::ops::ControlFlow;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputColumn {
    pub alias: String,
    pub member: String,
    pub semantic_type: DimensionType,
    pub granularity: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsedCube {
    pub name: String,
    /// Qualified identifiers as components, without SQL quoting delimiters.
    pub physical_tables: Vec<Vec<String>>,
}
fn fault() -> PlanError {
    PlanError::Internal("Result metadata does not correspond to the planned projection".into())
}

pub(crate) fn metadata(
    sql: &str,
    symbols: Vec<Rc<MemberSymbol>>,
    model: &Model,
) -> Result<(Vec<OutputColumn>, Vec<UsedCube>), PlanError> {
    let statements = Parser::parse_sql(&GenericDialect {}, sql).map_err(|_| fault())?;
    let [Statement::Query(query)] = statements.as_slice() else {
        return Err(fault());
    };
    let SetExpr::Select(select) = query.body.as_ref() else {
        return Err(fault());
    };
    let mut columns = vec![];
    for item in &select.projection {
        let SelectItem::ExprWithAlias { alias, .. } = item else {
            return Err(fault());
        };
        let mut matching = symbols.iter().filter(|m| m.alias() == alias.value);
        let member = matching.next().ok_or_else(fault)?;
        if matching.any(|m| m.full_name() != member.full_name()) {
            return Err(fault());
        }
        let name = member
            .time_dimension_behind_references()
            .map(|d| d.base_symbol().full_name())
            .unwrap_or_else(|| member.full_name());
        let (cube, field) = name.split_once('.').ok_or_else(fault)?;
        let cube = model.cube(cube).ok_or_else(fault)?;
        let semantic_type = if cube.measures.iter().any(|m| m.name == field) {
            DimensionType::Number
        } else {
            cube.dimensions
                .iter()
                .find(|d| d.name == field)
                .map(|d| d.dimension_type)
                .ok_or_else(fault)?
        };
        let granularity = member
            .time_dimension_behind_references()
            .and_then(|d| d.granularity().clone());
        columns.push(OutputColumn {
            alias: alias.value.clone(),
            member: name,
            semantic_type,
            granularity,
        });
    }
    let mut tables = Tables::default();
    let _ = statements.visit(&mut tables);
    let mut used = vec![];
    let alias_of = |cube: &pp_cube_model::Cube| {
        cubesqlplanner::planner::sql_templates::PlanSqlTemplates::alias_name(
            cube.sql_alias.as_deref().unwrap_or(&cube.name),
        )
    };
    let cube_aliases: Vec<String> = model.cubes.iter().map(alias_of).collect();
    for cube in &model.cubes {
        let expected_alias = alias_of(cube);

        if !tables.0.iter().any(|(_, a)| {
            a.as_deref()
                .is_some_and(|a| is_alias_of(a, &expected_alias, &cube_aliases))
        }) {
            continue;
        }

        let source = if let Some(table) = &cube.sql_table {
            format!("SELECT * FROM {table}")
        } else {
            cube.sql.clone().ok_or_else(fault)?
        };
        let source = Parser::parse_sql(&GenericDialect {}, &source).map_err(|_| fault())?;
        // The public facade validates first; the adapter can also be called directly.
        // Do not classify CTE/derived aliases from an out-of-baseline cube as tables.
        let [Statement::Query(source_query)] = source.as_slice() else {
            return Err(fault());
        };
        let SetExpr::Select(source_select) = source_query.body.as_ref() else {
            return Err(fault());
        };
        if source_query.with.is_some()
            || source_select
                .from
                .iter()
                .any(|t| !matches!(&t.relation, TableFactor::Table { .. }))
        {
            return Err(fault());
        }
        let mut physical = Tables::default();
        let _ = source.visit(&mut physical);
        if tables.0.iter().any(|(name, alias)| {
            alias
                .as_deref()
                .is_some_and(|a| is_alias_of(a, &expected_alias, &cube_aliases))
                && (name.is_empty() || physical.0.iter().any(|(p, _)| p == name))
        }) {
            let mut physical_tables: Vec<_> = physical.0.into_iter().map(|(n, _)| n).collect();
            physical_tables.sort();
            physical_tables.dedup();
            used.push(UsedCube {
                name: cube.name.clone(),
                physical_tables,
            });
        }
    }
    used.sort_by(|a, b| a.name.cmp(&b.name));
    Ok((columns, used))
}
/// Whether `alias` is how the planner names the cube aliased `expected`: as it is,
/// or as `<cube>_key_<expected>` inside the subquery of distinct keys it writes for
/// a measure of `<cube>` over repeated rows.
fn is_alias_of(alias: &str, expected: &str, cube_aliases: &[String]) -> bool {
    alias == expected
        || alias
            .strip_suffix(expected)
            .and_then(|rest| rest.strip_suffix("_key_"))
            .is_some_and(|fact| cube_aliases.iter().any(|known| known == fact))
}

#[derive(Default)]
struct Tables(Vec<(Vec<String>, Option<String>)>);
impl Visitor for Tables {
    type Break = ();
    fn pre_visit_table_factor(&mut self, t: &TableFactor) -> ControlFlow<()> {
        match t {
            TableFactor::Table { name, alias, .. } => self.0.push((
                name.0
                    .iter()
                    .map(|part| {
                        part.as_ident()
                            .map(|i| i.value.clone())
                            .unwrap_or_else(|| part.to_string())
                    })
                    .collect(),
                alias.as_ref().map(|a| a.name.value.clone()),
            )),
            TableFactor::Derived { alias, .. } => self
                .0
                .push((vec![], alias.as_ref().map(|a| a.name.value.clone()))),
            _ => {}
        }
        ControlFlow::Continue(())
    }
}
