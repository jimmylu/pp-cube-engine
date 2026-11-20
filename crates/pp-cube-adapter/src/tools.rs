//! The tool objects the planner calls back into: templates, dialect
//! functions, expression compilation and the join tree.

use std::any::Any;
use std::collections::HashMap;
use std::rc::Rc;

use cubenativeutils::CubeError;
use cubesqlplanner::cube_bridge::base_query_options::FilterValue;
use cubesqlplanner::cube_bridge::base_tools::BaseTools;
use cubesqlplanner::cube_bridge::driver_tools::DriverTools;
use cubesqlplanner::cube_bridge::join_definition::{JoinDefinition, JoinDefinitionStatic};
use cubesqlplanner::cube_bridge::join_graph::JoinGraph;
use cubesqlplanner::cube_bridge::join_hints::JoinHintItem;
use cubesqlplanner::cube_bridge::join_item::JoinItem;
use cubesqlplanner::cube_bridge::member_sql::{CompiledMemberTemplate, MemberSql};
use cubesqlplanner::cube_bridge::pre_aggregation_obj::PreAggregationObj;
use cubesqlplanner::cube_bridge::query_context::QueryContextData;
use cubesqlplanner::cube_bridge::security_context::SecurityContext;
use cubesqlplanner::cube_bridge::sql_templates_render::SqlTemplatesRender;
use cubesqlplanner::cube_bridge::sql_utils::SqlUtils;
use pp_cube_dialect::{Dialect, DialectError};

use crate::expression::Expression;
use crate::schema::Evaluator;
use crate::unavailable;

/// A missing template is the kernel's gap, not the caller's mistake.
fn dialect_error(error: DialectError) -> CubeError {
    CubeError::internal(error.to_string())
}

pub(crate) struct Templates(pub(crate) &'static Dialect);

impl SqlTemplatesRender for Templates {
    fn contains_template(&self, template_name: &str) -> bool {
        self.0.template(template_name).is_some()
    }

    fn render_template(&self, name: &str, ctx: minijinja::Value) -> Result<String, CubeError> {
        self.0.render(name, ctx).map_err(dialect_error)
    }

    fn get_template(&self, template_name: &str) -> Result<&String, CubeError> {
        self.0.template(template_name).ok_or_else(|| {
            dialect_error(DialectError::UnknownTemplate {
                dialect: self.0.name(),
                template: template_name.to_owned(),
            })
        })
    }
}

pub(crate) struct Driver {
    dialect: &'static Dialect,
    templates: Rc<Templates>,
}

impl Driver {
    pub(crate) fn new(dialect: &'static Dialect) -> Self {
        Self {
            dialect,
            templates: Rc::new(Templates(dialect)),
        }
    }
}

impl DriverTools for Driver {
    fn convert_tz(&self, _field: String) -> Result<String, CubeError> {
        unavailable("the dialect function convert_tz")
    }

    fn time_grouped_column(
        &self,
        _granularity: String,
        _dimension: String,
    ) -> Result<String, CubeError> {
        unavailable("the dialect function time_grouped_column")
    }

    fn sql_templates(&self) -> Result<Rc<dyn SqlTemplatesRender>, CubeError> {
        Ok(self.templates.clone())
    }

    fn timestamp_precision(&self) -> Result<u32, CubeError> {
        unavailable("the dialect setting timestamp_precision")
    }

    fn time_stamp_cast(&self, _field: String) -> Result<String, CubeError> {
        unavailable("the dialect function time_stamp_cast")
    }

    fn date_time_cast(&self, _field: String) -> Result<String, CubeError> {
        unavailable("the dialect function date_time_cast")
    }

    fn in_db_time_zone(&self, _date: String) -> Result<String, CubeError> {
        unavailable("the dialect function in_db_time_zone")
    }

    fn get_allocated_params(&self) -> Result<Vec<String>, CubeError> {
        Ok(Vec::new())
    }

    fn should_reuse_params(&self) -> Result<bool, CubeError> {
        Ok(self.dialect.reuses_parameters())
    }

    fn subtract_interval(&self, _date: String, _interval: String) -> Result<String, CubeError> {
        unavailable("the dialect function subtract_interval")
    }

    fn add_interval(&self, _date: String, _interval: String) -> Result<String, CubeError> {
        unavailable("the dialect function add_interval")
    }

    fn interval_string(&self, _interval: String) -> Result<String, CubeError> {
        unavailable("the dialect function interval_string")
    }

    fn add_timestamp_interval(
        &self,
        _date: String,
        _interval: String,
    ) -> Result<String, CubeError> {
        unavailable("the dialect function add_timestamp_interval")
    }

    fn interval_and_minimal_time_unit(&self, _interval: String) -> Result<Vec<String>, CubeError> {
        unavailable("the dialect function interval_and_minimal_time_unit")
    }

    fn hll_init(&self, _sql: String) -> Result<String, CubeError> {
        unavailable("the dialect function hll_init")
    }

    fn hll_merge(&self, _sql: String) -> Result<String, CubeError> {
        unavailable("the dialect function hll_merge")
    }

    fn hll_cardinality_merge(&self, _sql: String) -> Result<String, CubeError> {
        unavailable("the dialect function hll_cardinality_merge")
    }

    fn count_distinct_approx(&self, _sql: String) -> Result<String, CubeError> {
        unavailable("the dialect function count_distinct_approx")
    }

    fn support_generated_series_for_custom_td(&self) -> Result<bool, CubeError> {
        unavailable("the dialect setting support_generated_series_for_custom_td")
    }

    fn date_bin(
        &self,
        _interval: String,
        _source: String,
        _origin: String,
    ) -> Result<String, CubeError> {
        unavailable("the dialect function date_bin")
    }

    fn as_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

pub(crate) struct Tools {
    evaluator: Rc<Evaluator>,
    driver: Rc<Driver>,
    joins: Rc<Joins>,
}

impl Tools {
    pub(crate) fn new(
        evaluator: Rc<Evaluator>,
        dialect: &'static Dialect,
        joins: Rc<Joins>,
    ) -> Self {
        Self {
            evaluator,
            driver: Rc::new(Driver::new(dialect)),
            joins,
        }
    }
}

impl BaseTools for Tools {
    fn driver_tools(&self, _external: bool) -> Result<Rc<dyn DriverTools>, CubeError> {
        Ok(self.driver.clone())
    }

    fn sql_templates(&self) -> Result<Rc<dyn SqlTemplatesRender>, CubeError> {
        self.driver.sql_templates()
    }

    fn sql_utils_for_rust(&self) -> Result<Rc<dyn SqlUtils>, CubeError> {
        Ok(Rc::new(Opaque))
    }

    fn get_allocated_params(&self) -> Result<Vec<FilterValue>, CubeError> {
        Ok(Vec::new())
    }

    fn all_cube_members(&self, path: String) -> Result<Vec<String>, CubeError> {
        Ok(self.evaluator.members_of(&path))
    }

    fn interval_and_minimal_time_unit(&self, _interval: String) -> Result<Vec<String>, CubeError> {
        unavailable("interval parsing")
    }

    fn get_pre_aggregation_by_name(
        &self,
        _cube_name: String,
        _name: String,
    ) -> Result<Rc<dyn PreAggregationObj>, CubeError> {
        unavailable("pre-aggregations")
    }

    fn pre_aggregation_table_name(
        &self,
        _cube_name: String,
        _name: String,
    ) -> Result<String, CubeError> {
        unavailable("pre-aggregations")
    }

    fn join_tree_for_hints(
        &self,
        hints: Vec<JoinHintItem>,
    ) -> Result<Rc<dyn JoinDefinition>, CubeError> {
        self.joins.build_join(hints)
    }

    fn try_join_tree_for_hints(
        &self,
        hints: Vec<JoinHintItem>,
    ) -> Result<Vec<Rc<dyn JoinDefinition>>, CubeError> {
        Ok(vec![self.joins.build_join(hints)?])
    }

    fn compile_member_sql(
        &self,
        member_sql: Rc<dyn MemberSql>,
        _security_context: Rc<dyn SecurityContext>,
        _arg_names: Vec<String>,
    ) -> Result<CompiledMemberTemplate, CubeError> {
        let expression = member_sql.as_any().downcast::<Expression>().map_err(|_| {
            CubeError::internal("an expression that the adapter did not create".to_owned())
        })?;
        Ok(expression.compiled())
    }

    fn evaluate_query_context_condition(
        &self,
        _condition: Rc<dyn MemberSql>,
        _cube_name: String,
        _query_context: QueryContextData,
    ) -> Result<bool, CubeError> {
        unavailable("query context conditions")
    }

    fn as_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

/// The join tree of a query. Only a query over one cube has one so far.
pub(crate) struct Joins;

impl JoinGraph for Joins {
    fn build_join(
        &self,
        cubes_to_join: Vec<JoinHintItem>,
    ) -> Result<Rc<dyn JoinDefinition>, CubeError> {
        let mut cubes = cubes_to_join.iter().flat_map(|hint| match hint {
            JoinHintItem::Single(cube) => std::slice::from_ref(cube),
            JoinHintItem::Vector(cubes) => cubes.as_slice(),
        });
        let root = cubes
            .next()
            .ok_or_else(|| CubeError::internal("a join tree was asked for no cube".to_owned()))?;
        if cubes.any(|cube| cube != root) {
            return unavailable("a query over more than one cube");
        }
        Ok(Rc::new(SingleCube(JoinDefinitionStatic {
            root: root.clone(),
            multiplication_factor: HashMap::from([(root.clone(), false)]),
        })))
    }

    fn as_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

struct SingleCube(JoinDefinitionStatic);

impl JoinDefinition for SingleCube {
    fn static_data(&self) -> &JoinDefinitionStatic {
        &self.0
    }

    fn joins(&self) -> Result<Vec<Rc<dyn JoinItem>>, CubeError> {
        Ok(Vec::new())
    }

    fn as_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

/// Stands in for the two objects the planner only passes through.
pub(crate) struct Opaque;

impl SecurityContext for Opaque {
    fn as_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

impl SqlUtils for Opaque {
    fn as_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}
