//! The model as the planner reads it: lookups by path, and one definition
//! object per cube, measure and dimension.

use std::any::Any;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use cubenativeutils::CubeError;
use cubesqlplanner::cube_bridge::case_variant::CaseVariant;
use cubesqlplanner::cube_bridge::cube_definition::{CubeDefinition, CubeDefinitionStatic};
use cubesqlplanner::cube_bridge::dimension_definition::{
    DimensionDefinition, DimensionDefinitionStatic,
};
use cubesqlplanner::cube_bridge::evaluator::{CubeEvaluator, CubeEvaluatorStatic};
use cubesqlplanner::cube_bridge::geo_item::GeoItem;
use cubesqlplanner::cube_bridge::granularity_definition::{
    GranularityDefinition, GranularityDefinitionStatic,
};
use cubesqlplanner::cube_bridge::join_item_definition::{
    JoinItemDefinition, JoinItemDefinitionStatic,
};
use cubesqlplanner::cube_bridge::measure_definition::{MeasureDefinition, MeasureDefinitionStatic};
use cubesqlplanner::cube_bridge::member_order_by::MemberOrderBy;
use cubesqlplanner::cube_bridge::member_sql::MemberSql;
use cubesqlplanner::cube_bridge::multi_stage_filter::MultiStageFilterReferences;
use cubesqlplanner::cube_bridge::multi_stage_grain::MultiStageGrainReferences;
use cubesqlplanner::cube_bridge::pre_aggregation_description::PreAggregationDescription;
use cubesqlplanner::cube_bridge::segment_definition::{SegmentDefinition, SegmentDefinitionStatic};
use cubesqlplanner::cube_bridge::struct_with_sql_member::StructWithSqlMember;
use cubesqlplanner::cube_bridge::timeshift_definition::TimeShiftDefinition;
use cubesqlplanner::cube_bridge::view_filter_definition::ViewFilterDefinition;
use pp_cube_model::{
    Condition, Cube, DateValue, DimensionType, Filter, Join, MeasureType, Model, Operator, Query,
    Relationship, TimeDimension,
};

use crate::expression::Expression;
use crate::options::{dates_taken, operator_name};
use crate::unavailable;

/// What a filter is applied to, as far as an operator cares.
#[derive(Clone, Copy)]
enum MemberKind {
    Dimension(DimensionType),
    Measure,
}

impl MemberKind {
    fn described(self) -> &'static str {
        match self {
            Self::Dimension(DimensionType::String) => "a string dimension",
            Self::Dimension(DimensionType::Number) => "a number dimension",
            Self::Dimension(DimensionType::Boolean) => "a boolean dimension",
            Self::Dimension(DimensionType::Time) => "a time dimension",
            Self::Measure => "a measure",
        }
    }

    /// Which operators each kind of member takes, as Cube documents them. A
    /// time dimension takes the date operators and no value comparison: the
    /// planner would not read the value of one in the query's timezone.
    fn takes(self, operator: Operator) -> bool {
        use Operator::{
            AfterDate, AfterOrOnDate, BeforeDate, BeforeOrOnDate, Contains, EndsWith, Equals, Gt,
            Gte, InDateRange, Lt, Lte, NotContains, NotEndsWith, NotEquals, NotInDateRange, NotSet,
            NotStartsWith, Set, StartsWith,
        };
        let time = matches!(self, Self::Dimension(DimensionType::Time));
        match operator {
            Set | NotSet => true,
            Equals | NotEquals => !time,
            InDateRange | NotInDateRange | BeforeDate | BeforeOrOnDate | AfterDate
            | AfterOrOnDate => time,
            Contains | NotContains | StartsWith | NotStartsWith | EndsWith | NotEndsWith => {
                matches!(self, Self::Dimension(DimensionType::String))
            }
            Gt | Gte | Lt | Lte => {
                matches!(self, Self::Dimension(DimensionType::Number) | Self::Measure)
            }
        }
    }
}

fn expression(text: &str) -> Result<Rc<dyn MemberSql>, CubeError> {
    Ok(Expression::compile(text)?)
}

fn optional_expression(text: Option<&String>) -> Result<Option<Rc<dyn MemberSql>>, CubeError> {
    text.map(|text| expression(text)).transpose()
}

/// The planner's view of the whole model.
pub(crate) struct Evaluator {
    model: Arc<Model>,
    statics: CubeEvaluatorStatic,
}

impl Evaluator {
    pub(crate) fn new(model: Arc<Model>) -> Self {
        let primary_keys = model
            .cubes
            .iter()
            .map(|cube| {
                let keys = cube
                    .dimensions
                    .iter()
                    .filter(|dimension| dimension.primary_key);
                (
                    cube.name.clone(),
                    keys.map(|dimension| dimension.name.clone()).collect(),
                )
            })
            .collect::<HashMap<_, Vec<_>>>();
        Self {
            model,
            statics: CubeEvaluatorStatic { primary_keys },
        }
    }

    /// The names of everything a cube declares, for the planner's cube symbol.
    pub(crate) fn members_of(&self, cube: &str) -> Vec<String> {
        let Some(cube) = self.model.cube(cube) else {
            return Vec::new();
        };
        let dimensions = cube.dimensions.iter().map(|dimension| &dimension.name);
        let measures = cube.measures.iter().map(|measure| &measure.name);
        let segments = cube.segments.iter().map(|segment| &segment.name);
        dimensions
            .chain(measures)
            .chain(segments)
            .cloned()
            .collect()
    }

    /// Fails for the first thing in `query` that cannot be compiled against
    /// the model: a member it does not declare, or a filter whose operator
    /// does not fit its member. The planner refuses an unknown member too,
    /// with a message that leaves out the cube; it does not check the fit.
    pub(crate) fn check(&self, query: &Query) -> Result<(), CubeError> {
        for measure in &query.measures {
            self.measure_index(measure)?;
        }
        for dimension in &query.dimensions {
            self.dimension_index(dimension)?;
        }
        for segment in &query.segments {
            self.segment_index(segment)?;
        }
        for filter in &query.filters {
            self.check_filter(filter)?;
        }
        for time_dimension in &query.time_dimensions {
            self.check_time_dimension(time_dimension)?;
        }
        // The planner drops an order on anything else without a word. A time
        // dimension is ordered by its member, and only where it is grouped by.
        for order in query.order.iter().flatten() {
            let selected = query.measures.contains(&order.member)
                || query.dimensions.contains(&order.member)
                || query.time_dimensions.iter().any(|time_dimension| {
                    time_dimension.dimension == order.member && time_dimension.granularity.is_some()
                });
            if !selected {
                return Err(CubeError::user(format!(
                    "Order member `{}` is not among the query's measures and dimensions",
                    order.member
                )));
            }
        }
        Ok(())
    }

    fn check_time_dimension(&self, time_dimension: &TimeDimension) -> Result<(), CubeError> {
        let member = &time_dimension.dimension;
        let kind = self.member_kind(member)?;
        if !matches!(kind, MemberKind::Dimension(DimensionType::Time)) {
            return Err(CubeError::user(format!(
                "`{member}` is {}; `timeDimensions` takes time dimensions",
                kind.described()
            )));
        }
        // The planner drops one that asks for nothing without a word.
        if time_dimension.granularity.is_none() && time_dimension.date_range.is_none() {
            return Err(CubeError::user(format!(
                "Time dimension `{member}` asks for neither a granularity nor a date range"
            )));
        }
        Ok(())
    }

    fn check_filter(&self, filter: &Filter) -> Result<(), CubeError> {
        match filter {
            Filter::And(filters) | Filter::Or(filters) => filters
                .iter()
                .try_for_each(|filter| self.check_filter(filter)),
            Filter::Condition(condition) => self.check_condition(condition),
        }
    }

    fn check_condition(&self, condition: &Condition) -> Result<(), CubeError> {
        let Condition {
            member,
            operator,
            values,
        } = condition;
        let kind = self.member_kind(member)?;
        let name = operator_name(*operator);
        if !kind.takes(*operator) {
            return Err(CubeError::user(format!(
                "Operator `{name}` does not fit `{member}`, {}",
                kind.described()
            )));
        }
        if let Some(count) = dates_taken(*operator) {
            if values.len() != count {
                let dates = if count == 1 { "one date" } else { "two dates" };
                return Err(CubeError::user(format!(
                    "Operator `{name}` on `{member}` takes {dates}"
                )));
            }
            return values.iter().try_for_each(|value| {
                DateValue::try_from(value.clone())
                    .map(drop)
                    .map_err(|message| {
                        CubeError::user(format!("Operator `{name}` on `{member}`: {message}"))
                    })
            });
        }
        let takes_values = !matches!(operator, Operator::Set | Operator::NotSet);
        match (takes_values, values.is_empty()) {
            (true, true) => Err(CubeError::user(format!(
                "Operator `{name}` on `{member}` needs at least one value"
            ))),
            (false, false) => Err(CubeError::user(format!(
                "Operator `{name}` on `{member}` takes no values"
            ))),
            _ => Ok(()),
        }
    }

    fn member_kind(&self, path: &str) -> Result<MemberKind, CubeError> {
        let (cube, name) = self.split(path)?;
        let cube = self.cube(cube);
        let dimension = cube
            .dimensions
            .iter()
            .find(|dimension| dimension.name == name);
        let measure = cube.measures.iter().any(|measure| measure.name == name);
        match (dimension, measure) {
            (Some(dimension), _) => Ok(MemberKind::Dimension(dimension.dimension_type)),
            (None, true) => Ok(MemberKind::Measure),
            (None, false) => Err(CubeError::user(format!(
                "`{path}` is not a dimension or a measure of the model"
            ))),
        }
    }

    fn cube_index(&self, name: &str) -> Result<usize, CubeError> {
        self.model
            .cubes
            .iter()
            .position(|cube| cube.name == name)
            .ok_or_else(|| CubeError::user(format!("Cube `{name}` is not in the model")))
    }

    /// Splits `cube.member` and finds the cube.
    fn split<'p>(&self, path: &'p str) -> Result<(usize, &'p str), CubeError> {
        let (cube, member) = path
            .split_once('.')
            .filter(|(_, member)| !member.contains('.'))
            .ok_or_else(|| CubeError::user(format!("`{path}` is not written `cube.member`")))?;
        Ok((self.cube_index(cube)?, member))
    }

    fn cube(&self, index: usize) -> &Cube {
        &self.model.cubes[index]
    }

    fn measure_index(&self, path: &str) -> Result<(usize, usize), CubeError> {
        let (cube, name) = self.split(path)?;
        let measures = &self.cube(cube).measures;
        let found = measures.iter().position(|measure| measure.name == name);
        found
            .map(|measure| (cube, measure))
            .ok_or_else(|| CubeError::user(format!("Measure `{path}` is not in the model")))
    }

    fn dimension_index(&self, path: &str) -> Result<(usize, usize), CubeError> {
        let (cube, name) = self.split(path)?;
        let dimensions = &self.cube(cube).dimensions;
        let found = dimensions
            .iter()
            .position(|dimension| dimension.name == name);
        found
            .map(|dimension| (cube, dimension))
            .ok_or_else(|| CubeError::user(format!("Dimension `{path}` is not in the model")))
    }

    fn segment_index(&self, path: &str) -> Result<(usize, usize), CubeError> {
        let (cube, name) = self.split(path)?;
        let segments = &self.cube(cube).segments;
        let found = segments.iter().position(|segment| segment.name == name);
        found
            .map(|segment| (cube, segment))
            .ok_or_else(|| CubeError::user(format!("Segment `{path}` is not in the model")))
    }

    fn has_member(&self, path: &[String], named: impl Fn(&Cube, &str) -> bool) -> bool {
        let [cube, member] = path else {
            return false;
        };
        self.model
            .cube(cube)
            .is_some_and(|cube| named(cube, member))
    }
}

impl CubeEvaluator for Evaluator {
    fn static_data(&self) -> &CubeEvaluatorStatic {
        &self.statics
    }

    fn parse_path(&self, path_type: String, path: String) -> Result<Vec<String>, CubeError> {
        match path_type.as_str() {
            "measure" | "measures" => self.measure_index(&path).map(drop)?,
            "dimension" | "dimensions" => self.dimension_index(&path).map(drop)?,
            "segment" | "segments" => self.segment_index(&path).map(drop)?,
            other => return unavailable(&format!("members of kind `{other}`")),
        }
        Ok(path.split('.').map(str::to_owned).collect())
    }

    fn measure_by_path(&self, path: String) -> Result<Rc<dyn MeasureDefinition>, CubeError> {
        let (cube, measure) = self.measure_index(&path)?;
        Ok(Rc::new(MeasureDef::new(self.model.clone(), cube, measure)))
    }

    fn dimension_by_path(&self, path: String) -> Result<Rc<dyn DimensionDefinition>, CubeError> {
        let (cube, dimension) = self.dimension_index(&path)?;
        Ok(Rc::new(DimensionDef::new(
            self.model.clone(),
            cube,
            dimension,
        )))
    }

    fn segment_by_path(&self, path: String) -> Result<Rc<dyn SegmentDefinition>, CubeError> {
        let (cube, segment) = self.segment_index(&path)?;
        let condition = Expression::compile(&self.cube(cube).segments[segment].sql)?;
        Ok(Rc::new(SegmentDef {
            condition,
            statics: SegmentDefinitionStatic {
                segment_type: None,
                owned_by_cube: None,
            },
        }))
    }

    fn cube_from_path(&self, path: String) -> Result<Rc<dyn CubeDefinition>, CubeError> {
        let cube = self.cube_index(&path)?;
        Ok(Rc::new(CubeDef::new(self.model.clone(), cube)))
    }

    fn is_measure(&self, path: Vec<String>) -> Result<bool, CubeError> {
        Ok(self.has_member(&path, |cube, name| {
            cube.measures.iter().any(|measure| measure.name == name)
        }))
    }

    fn is_dimension(&self, path: Vec<String>) -> Result<bool, CubeError> {
        Ok(self.has_member(&path, |cube, name| {
            cube.dimensions
                .iter()
                .any(|dimension| dimension.name == name)
        }))
    }

    fn is_segment(&self, path: Vec<String>) -> Result<bool, CubeError> {
        Ok(self.has_member(&path, |cube, name| {
            cube.segments.iter().any(|segment| segment.name == name)
        }))
    }

    fn cube_exists(&self, name: String) -> Result<bool, CubeError> {
        Ok(self.model.cube(&name).is_some())
    }

    // The planner asks for the predefined granularities too, and the query
    // type admits no other.
    fn resolve_granularity(
        &self,
        path: Vec<String>,
    ) -> Result<Rc<dyn GranularityDefinition>, CubeError> {
        let Some(granularity) = path.last() else {
            return Err(CubeError::internal(
                "a granularity without a path".to_owned(),
            ));
        };
        Ok(Rc::new(PredefinedGranularity(
            GranularityDefinitionStatic {
                interval: format!("1 {granularity}"),
                origin: None,
                offset: None,
            },
        )))
    }

    // The baseline has no pre-aggregations; the planner asks on every query.
    fn pre_aggregations_for_cube_as_array(
        &self,
        _cube_name: String,
    ) -> Result<Vec<Rc<dyn PreAggregationDescription>>, CubeError> {
        Ok(Vec::new())
    }

    fn has_pre_aggregation_description_by_name(&self) -> Result<bool, CubeError> {
        Ok(true)
    }

    fn pre_aggregation_description_by_name(
        &self,
        _cube_name: String,
        _name: String,
    ) -> Result<Option<Rc<dyn PreAggregationDescription>>, CubeError> {
        Ok(None)
    }

    fn evaluate_rollup_references(
        &self,
        _cube: String,
        _sql: Rc<dyn MemberSql>,
    ) -> Result<Vec<String>, CubeError> {
        unavailable("rollup references")
    }

    fn as_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

struct CubeDef {
    model: Arc<Model>,
    cube: usize,
    statics: CubeDefinitionStatic,
}

impl CubeDef {
    fn new(model: Arc<Model>, cube: usize) -> Self {
        let definition = &model.cubes[cube];
        let statics = CubeDefinitionStatic {
            name: definition.name.clone(),
            sql_alias: definition.sql_alias.clone(),
            is_view: None,
            is_calendar: None,
            join_map: None,
        };
        Self {
            model,
            cube,
            statics,
        }
    }

    fn definition(&self) -> &Cube {
        &self.model.cubes[self.cube]
    }
}

impl CubeDefinition for CubeDef {
    fn static_data(&self) -> &CubeDefinitionStatic {
        &self.statics
    }

    fn has_sql_table(&self) -> Result<bool, CubeError> {
        Ok(self.definition().sql_table.is_some())
    }

    fn sql_table(&self) -> Result<Option<Rc<dyn MemberSql>>, CubeError> {
        optional_expression(self.definition().sql_table.as_ref())
    }

    fn has_sql(&self) -> Result<bool, CubeError> {
        Ok(self.definition().sql.is_some())
    }

    fn sql(&self) -> Result<Option<Rc<dyn MemberSql>>, CubeError> {
        optional_expression(self.definition().sql.as_ref())
    }

    fn has_default_filters(&self) -> Result<bool, CubeError> {
        Ok(false)
    }

    fn default_filters(&self) -> Result<Option<Vec<Rc<dyn ViewFilterDefinition>>>, CubeError> {
        Ok(None)
    }

    fn has_joins(&self) -> Result<bool, CubeError> {
        Ok(!self.definition().joins.is_empty())
    }

    fn joins(&self) -> Result<Option<Vec<Rc<dyn JoinItemDefinition>>>, CubeError> {
        let joins = self.definition().joins.iter();
        let joins =
            joins.map(|join| Ok(Rc::new(JoinDef::new(join)?) as Rc<dyn JoinItemDefinition>));
        joins.collect::<Result<_, CubeError>>().map(Some)
    }

    fn as_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

struct MeasureDef {
    model: Arc<Model>,
    cube: usize,
    measure: usize,
    statics: MeasureDefinitionStatic,
}

impl MeasureDef {
    fn new(model: Arc<Model>, cube: usize, measure: usize) -> Self {
        // The names Cube's own compiler hands the planner.
        let measure_type = match model.cubes[cube].measures[measure].measure_type {
            MeasureType::Count => "count",
            MeasureType::CountDistinct => "countDistinct",
            MeasureType::Sum => "sum",
            MeasureType::Avg => "avg",
            MeasureType::Min => "min",
            MeasureType::Max => "max",
            MeasureType::Number => "number",
        };
        let statics = MeasureDefinitionStatic {
            measure_type: measure_type.to_owned(),
            owned_by_cube: None,
            multi_stage: None,
            reduce_by_references: None,
            add_group_by_references: None,
            group_by_references: None,
            time_shift_references: None,
            rolling_window: None,
            included: None,
        };
        Self {
            model,
            cube,
            measure,
            statics,
        }
    }

    fn definition(&self) -> &pp_cube_model::Measure {
        &self.model.cubes[self.cube].measures[self.measure]
    }
}

impl MeasureDefinition for MeasureDef {
    fn static_data(&self) -> &MeasureDefinitionStatic {
        &self.statics
    }

    fn has_sql(&self) -> Result<bool, CubeError> {
        Ok(self.definition().sql.is_some())
    }

    fn sql(&self) -> Result<Option<Rc<dyn MemberSql>>, CubeError> {
        optional_expression(self.definition().sql.as_ref())
    }

    fn has_case(&self) -> Result<bool, CubeError> {
        Ok(false)
    }

    fn case(&self) -> Result<Option<CaseVariant>, CubeError> {
        Ok(None)
    }

    fn has_filters(&self) -> Result<bool, CubeError> {
        Ok(!self.definition().filters.is_empty())
    }

    fn filters(&self) -> Result<Option<Vec<Rc<dyn StructWithSqlMember>>>, CubeError> {
        let filters = &self.definition().filters;
        if filters.is_empty() {
            return Ok(None);
        }
        let conditions = filters.iter().map(|filter| {
            let condition = Expression::compile(&filter.sql)?;
            Ok(Rc::new(MeasureCondition(condition)) as Rc<dyn StructWithSqlMember>)
        });
        conditions.collect::<Result<Vec<_>, CubeError>>().map(Some)
    }

    fn has_filter(&self) -> Result<bool, CubeError> {
        Ok(false)
    }

    fn filter(&self) -> Result<Option<Rc<dyn MultiStageFilterReferences>>, CubeError> {
        Ok(None)
    }

    fn has_grain(&self) -> Result<bool, CubeError> {
        Ok(false)
    }

    fn grain(&self) -> Result<Option<Rc<dyn MultiStageGrainReferences>>, CubeError> {
        Ok(None)
    }

    fn has_drill_filters(&self) -> Result<bool, CubeError> {
        Ok(false)
    }

    fn drill_filters(&self) -> Result<Option<Vec<Rc<dyn StructWithSqlMember>>>, CubeError> {
        Ok(None)
    }

    fn has_order_by(&self) -> Result<bool, CubeError> {
        Ok(false)
    }

    fn order_by(&self) -> Result<Option<Vec<Rc<dyn MemberOrderBy>>>, CubeError> {
        Ok(None)
    }

    fn has_mask_sql(&self) -> Result<bool, CubeError> {
        Ok(false)
    }

    fn mask_sql(&self) -> Result<Option<Rc<dyn MemberSql>>, CubeError> {
        Ok(None)
    }

    fn as_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

/// One condition of a filtered measure.
struct MeasureCondition(Rc<Expression>);

impl StructWithSqlMember for MeasureCondition {
    fn sql(&self) -> Result<Rc<dyn MemberSql>, CubeError> {
        Ok(self.0.clone())
    }

    fn as_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

struct SegmentDef {
    condition: Rc<Expression>,
    statics: SegmentDefinitionStatic,
}

impl SegmentDefinition for SegmentDef {
    fn static_data(&self) -> &SegmentDefinitionStatic {
        &self.statics
    }

    fn sql(&self) -> Result<Rc<dyn MemberSql>, CubeError> {
        Ok(self.condition.clone())
    }

    fn as_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

/// A join as its cube declares it. The condition is an expression of that cube.
pub(crate) struct JoinDef {
    condition: Rc<Expression>,
    statics: JoinItemDefinitionStatic,
}

impl JoinDef {
    pub(crate) fn new(join: &Join) -> Result<Self, CubeError> {
        let relationship = match join.relationship {
            Relationship::ManyToOne => "many_to_one",
            Relationship::OneToOne => "one_to_one",
        };
        Ok(Self {
            condition: Expression::compile(&join.sql)?,
            statics: JoinItemDefinitionStatic {
                name: join.name.clone(),
                alias: None,
                relationship: relationship.to_owned(),
            },
        })
    }
}

impl JoinItemDefinition for JoinDef {
    fn static_data(&self) -> &JoinItemDefinitionStatic {
        &self.statics
    }

    fn sql(&self) -> Result<Rc<dyn MemberSql>, CubeError> {
        Ok(self.condition.clone())
    }

    fn as_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

struct PredefinedGranularity(GranularityDefinitionStatic);

impl GranularityDefinition for PredefinedGranularity {
    fn static_data(&self) -> &GranularityDefinitionStatic {
        &self.0
    }

    fn sql(&self) -> Result<Option<Rc<dyn MemberSql>>, CubeError> {
        Ok(None)
    }

    fn has_sql(&self) -> Result<bool, CubeError> {
        Ok(false)
    }

    fn as_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

struct DimensionDef {
    model: Arc<Model>,
    cube: usize,
    dimension: usize,
    statics: DimensionDefinitionStatic,
}

impl DimensionDef {
    fn new(model: Arc<Model>, cube: usize, dimension: usize) -> Self {
        let definition = &model.cubes[cube].dimensions[dimension];
        let dimension_type = match definition.dimension_type {
            DimensionType::String => "string",
            DimensionType::Number => "number",
            DimensionType::Boolean => "boolean",
            DimensionType::Time => "time",
        };
        let statics = DimensionDefinitionStatic {
            dimension_type: dimension_type.to_owned(),
            owned_by_cube: None,
            multi_stage: None,
            add_group_by_references: None,
            sub_query: None,
            propagate_filters_to_sub_query: None,
            values: None,
            primary_key: Some(definition.primary_key),
            field_type: None,
            included: None,
        };
        Self {
            model,
            cube,
            dimension,
            statics,
        }
    }
}

impl DimensionDefinition for DimensionDef {
    fn static_data(&self) -> &DimensionDefinitionStatic {
        &self.statics
    }

    fn has_sql(&self) -> Result<bool, CubeError> {
        Ok(true)
    }

    fn sql(&self) -> Result<Option<Rc<dyn MemberSql>>, CubeError> {
        let definition = &self.model.cubes[self.cube].dimensions[self.dimension];
        expression(&definition.sql).map(Some)
    }

    fn has_case(&self) -> Result<bool, CubeError> {
        Ok(false)
    }

    fn case(&self) -> Result<Option<CaseVariant>, CubeError> {
        Ok(None)
    }

    fn has_latitude(&self) -> Result<bool, CubeError> {
        Ok(false)
    }

    fn latitude(&self) -> Result<Option<Rc<dyn GeoItem>>, CubeError> {
        Ok(None)
    }

    fn has_longitude(&self) -> Result<bool, CubeError> {
        Ok(false)
    }

    fn longitude(&self) -> Result<Option<Rc<dyn GeoItem>>, CubeError> {
        Ok(None)
    }

    fn has_time_shift(&self) -> Result<bool, CubeError> {
        Ok(false)
    }

    fn time_shift(&self) -> Result<Option<Vec<Rc<dyn TimeShiftDefinition>>>, CubeError> {
        Ok(None)
    }

    fn has_filter(&self) -> Result<bool, CubeError> {
        Ok(false)
    }

    fn filter(&self) -> Result<Option<Rc<dyn MultiStageFilterReferences>>, CubeError> {
        Ok(None)
    }

    fn has_mask_sql(&self) -> Result<bool, CubeError> {
        Ok(false)
    }

    fn mask_sql(&self) -> Result<Option<Rc<dyn MemberSql>>, CubeError> {
        Ok(None)
    }

    fn as_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}
