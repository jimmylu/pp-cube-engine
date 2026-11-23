//! The tool objects the planner calls back into: templates, dialect
//! functions, expression compilation and the join tree.

use std::any::Any;
use std::rc::Rc;

use chrono::{NaiveDateTime, Offset, TimeZone};
use chrono_tz::Tz;
use cubenativeutils::CubeError;
use cubesqlplanner::cube_bridge::base_query_options::FilterValue;
use cubesqlplanner::cube_bridge::base_tools::BaseTools;
use cubesqlplanner::cube_bridge::driver_tools::DriverTools;
use cubesqlplanner::cube_bridge::join_definition::JoinDefinition;
use cubesqlplanner::cube_bridge::join_graph::JoinGraph;
use cubesqlplanner::cube_bridge::join_hints::JoinHintItem;
use cubesqlplanner::cube_bridge::member_sql::{CompiledMemberTemplate, MemberSql};
use cubesqlplanner::cube_bridge::pre_aggregation_obj::PreAggregationObj;
use cubesqlplanner::cube_bridge::query_context::QueryContextData;
use cubesqlplanner::cube_bridge::security_context::SecurityContext;
use cubesqlplanner::cube_bridge::sql_templates_render::SqlTemplatesRender;
use cubesqlplanner::cube_bridge::sql_utils::SqlUtils;
use minijinja::context;
use pp_cube_dialect::{Dialect, DialectError};

use crate::expression::Expression;
use crate::joins::Joins;
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
    zone: Tz,
}

impl Driver {
    pub(crate) fn new(dialect: &'static Dialect, zone: Tz) -> Self {
        Self {
            dialect,
            templates: Rc::new(Templates(dialect)),
            zone,
        }
    }

    /// Renders one of the dialect's functions, kept as `driver/` templates.
    fn function(&self, name: &str, arguments: minijinja::Value) -> Result<String, CubeError> {
        self.dialect.render(name, arguments).map_err(dialect_error)
    }

    /// Adds `interval` to `date` or subtracts it, one unit at a time, coarsest
    /// first: an interval literal of these engines carries a single unit.
    fn apply_interval(
        &self,
        name: &str,
        date: String,
        interval: &str,
    ) -> Result<String, CubeError> {
        let mut date = date;
        for (value, unit) in interval_parts(interval)? {
            let interval = self.function("driver/interval_string", context! { value, unit })?;
            date = self.function(name, context! { date, interval })?;
        }
        Ok(date)
    }
}

/// The parts of an interval such as `1 year 2 month`, one per unit, coarsest
/// first, as Cube's `splitSqlInterval` (cubejs-backend-shared/src/time.ts) gives them.
fn interval_parts(interval: &str) -> Result<Vec<(i64, String)>, CubeError> {
    const UNITS: [&str; 9] = [
        "year",
        "quarter",
        "month",
        "week",
        "day",
        "hour",
        "minute",
        "second",
        "millisecond",
    ];
    let malformed = || CubeError::internal(format!("`{interval}` is not an interval"));
    let words: Vec<&str> = interval.split_whitespace().collect();
    if words.is_empty() || !words.len().is_multiple_of(2) {
        return Err(malformed());
    }
    let mut parts = Vec::with_capacity(words.len() / 2);
    for pair in words.chunks(2) {
        let value: i64 = pair[0].parse().map_err(|_| malformed())?;
        let unit = pair[1].to_lowercase();
        let unit = unit.strip_suffix('s').unwrap_or(&unit);
        let rank = UNITS
            .iter()
            .position(|known| *known == unit)
            .ok_or_else(malformed)?;
        parts.push((rank, value, unit.to_owned()));
    }
    parts.sort_by_key(|(rank, _, _)| *rank);
    Ok(parts
        .into_iter()
        .map(|(_, value, unit)| (value, unit))
        .collect())
}

impl DriverTools for Driver {
    // The zone's name comes from the timezone database, not from the caller's
    // text, so it is safe inside the quotes of the template.
    fn convert_tz(&self, field: String) -> Result<String, CubeError> {
        self.function(
            "driver/convert_tz",
            context! { field, timezone => self.zone.name() },
        )
    }

    fn time_grouped_column(
        &self,
        granularity: String,
        dimension: String,
    ) -> Result<String, CubeError> {
        self.function(
            "driver/time_grouped_column",
            context! { granularity, dimension },
        )
    }

    fn sql_templates(&self) -> Result<Rc<dyn SqlTemplatesRender>, CubeError> {
        Ok(self.templates.clone())
    }

    fn timestamp_precision(&self) -> Result<u32, CubeError> {
        Ok(self.dialect.timestamp_precision())
    }

    fn time_stamp_cast(&self, field: String) -> Result<String, CubeError> {
        self.function("driver/time_stamp_cast", context! { value => field })
    }

    fn date_time_cast(&self, field: String) -> Result<String, CubeError> {
        self.function("driver/date_time_cast", context! { value => field })
    }

    /// A timestamp of the query's timezone as UTC text, the way Cube's
    /// `localTimestampToUtc` (packages/cubejs-backend-shared/src/time.ts)
    /// does it. The offset is looked up as if the local time were UTC, so
    /// within a clock change's width of the change it is the offset of the
    /// other side. That is kept: the kernel binds what Cube binds.
    fn in_db_time_zone(&self, date: String) -> Result<String, CubeError> {
        const FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.3f";
        let local = NaiveDateTime::parse_from_str(&date, FORMAT).map_err(|_| {
            CubeError::internal(format!("`{date}` is not a timestamp the adapter wrote"))
        })?;
        let offset = self.zone.offset_from_utc_datetime(&local).fix();
        Ok(format!("{}Z", (local - offset).format(FORMAT)))
    }

    fn get_allocated_params(&self) -> Result<Vec<String>, CubeError> {
        Ok(Vec::new())
    }

    fn should_reuse_params(&self) -> Result<bool, CubeError> {
        Ok(self.dialect.reuses_parameters())
    }

    fn subtract_interval(&self, date: String, interval: String) -> Result<String, CubeError> {
        self.apply_interval("driver/subtract_interval", date, &interval)
    }

    fn add_interval(&self, date: String, interval: String) -> Result<String, CubeError> {
        self.apply_interval("driver/add_interval", date, &interval)
    }

    fn interval_string(&self, interval: String) -> Result<String, CubeError> {
        let parts = interval_parts(&interval)?;
        let [(value, unit)] = parts.as_slice() else {
            return Err(CubeError::internal(format!(
                "`{interval}` is not an interval of one unit"
            )));
        };
        self.function("driver/interval_string", context! { value, unit })
    }

    fn add_timestamp_interval(&self, date: String, interval: String) -> Result<String, CubeError> {
        self.add_interval(date, interval)
    }

    /// Cube's `intervalAndMinimalTimeUnit` (BaseQuery.ts): the interval as it is
    /// and the finest unit a difference in it has to be counted in.
    fn interval_and_minimal_time_unit(&self, interval: String) -> Result<Vec<String>, CubeError> {
        let lower = interval.to_lowercase();
        let has = |unit: &str| lower.contains(unit);
        let unit = if has("second") {
            "second"
        } else if has("minute") {
            "minute"
        } else if has("hour") {
            "hour"
        } else if has("day") || has("week") {
            "day"
        } else if has("month") || has("quarter") {
            "month"
        } else {
            "year"
        };
        Ok(vec![interval, unit.to_owned()])
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
        Ok(self.dialect.generates_time_series())
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
        zone: Tz,
        joins: Rc<Joins>,
    ) -> Self {
        Self {
            evaluator,
            driver: Rc::new(Driver::new(dialect, zone)),
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
        Ok(self.joins.try_build(&hints)?.into_iter().collect())
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
