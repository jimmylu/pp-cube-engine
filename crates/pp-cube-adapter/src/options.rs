//! A query in the form the planner takes.

use std::any::Any;
use std::rc::Rc;

use cubenativeutils::CubeError;
use cubesqlplanner::cube_bridge::base_query_options::{
    BaseQueryOptions, BaseQueryOptionsStatic, FilterItem, FilterValue, OrderByItem,
};
use cubesqlplanner::cube_bridge::base_tools::BaseTools;
use cubesqlplanner::cube_bridge::evaluator::CubeEvaluator;
use cubesqlplanner::cube_bridge::join_graph::JoinGraph;
use cubesqlplanner::cube_bridge::join_hints::JoinHintItem;
use cubesqlplanner::cube_bridge::options_member::OptionsMember;
use cubesqlplanner::cube_bridge::security_context::SecurityContext;
use cubesqlplanner::cube_bridge::subquery_join::SubqueryJoin;
use pp_cube_model::{Direction, Filter, Operator, Query};

/// The operator as Cube's query format writes it.
pub(crate) fn operator_name(operator: Operator) -> &'static str {
    match operator {
        Operator::Equals => "equals",
        Operator::NotEquals => "notEquals",
        Operator::Contains => "contains",
        Operator::NotContains => "notContains",
        Operator::StartsWith => "startsWith",
        Operator::NotStartsWith => "notStartsWith",
        Operator::EndsWith => "endsWith",
        Operator::NotEndsWith => "notEndsWith",
        Operator::Gt => "gt",
        Operator::Gte => "gte",
        Operator::Lt => "lt",
        Operator::Lte => "lte",
        Operator::Set => "set",
        Operator::NotSet => "notSet",
    }
}

fn filter_item(filter: &Filter) -> FilterItem {
    let mut item = FilterItem {
        or: None,
        and: None,
        member: None,
        dimension: None,
        operator: None,
        values: None,
    };
    match filter {
        Filter::And(filters) => item.and = Some(filters.iter().map(filter_item).collect()),
        Filter::Or(filters) => item.or = Some(filters.iter().map(filter_item).collect()),
        Filter::Condition(condition) => {
            item.member = Some(condition.member.clone());
            item.operator = Some(operator_name(condition.operator).to_owned());
            let values = condition.values.iter().cloned().map(FilterValue::Str);
            item.values = Some(values.collect()).filter(|values: &Vec<_>| !values.is_empty());
        }
    }
    item
}

pub(crate) struct Options {
    measures: Vec<OptionsMember>,
    dimensions: Vec<OptionsMember>,
    segments: Vec<OptionsMember>,
    evaluator: Rc<dyn CubeEvaluator>,
    tools: Rc<dyn BaseTools>,
    joins: Rc<dyn JoinGraph>,
    security_context: Rc<dyn SecurityContext>,
    statics: BaseQueryOptionsStatic,
}

impl Options {
    pub(crate) fn new(
        query: &Query,
        evaluator: Rc<dyn CubeEvaluator>,
        tools: Rc<dyn BaseTools>,
        joins: Rc<dyn JoinGraph>,
        security_context: Rc<dyn SecurityContext>,
    ) -> Self {
        let members = |names: &[String]| {
            names
                .iter()
                .cloned()
                .map(OptionsMember::MemberName)
                .collect()
        };
        let order = query.order.iter().map(|order| OrderByItem {
            id: order.member.clone(),
            desc: Some(order.direction == Direction::Desc),
        });
        Self {
            measures: members(&query.measures),
            dimensions: members(&query.dimensions),
            segments: members(&query.segments),
            evaluator,
            tools,
            joins,
            security_context,
            // Every field is named, so a field the planner adds shows up here
            // as a compile error when the pin moves.
            statics: BaseQueryOptionsStatic {
                time_dimensions: None,
                timezone: None,
                filters: Some(query.filters.iter().map(filter_item).collect())
                    .filter(|filters: &Vec<_>| !filters.is_empty()),
                order: Some(order.collect()).filter(|order: &Vec<_>| !order.is_empty()),
                limit: None,
                // The planner reads the row limit from here, not from `limit`.
                row_limit: query.limit.map(|limit| limit.to_string()),
                offset: query.offset.map(|offset| offset.to_string()),
                ungrouped: None,
                export_annotated_sql: false,
                pre_aggregation_query: None,
                pre_aggregations_match_only: None,
                use_original_sql_pre_aggregations_in_pre_aggregation: None,
                total_query: None,
                cubestore_support_multistage: None,
                multi_stage_leaf_merge: None,
                multi_stage_scan_sharing: None,
                cubestore_union_full_key_aggregate: None,
                union_full_key_aggregate: None,
                max_multi_stage_depth: None,
                max_multi_stage_stages: None,
                max_member_resolution_depth: None,
                disable_external_pre_aggregations: false,
                pre_aggregation_id: None,
                convert_tz_for_raw_time_dimension: None,
                masked_members: None,
                member_to_alias: None,
                data_source: None,
            },
        }
    }
}

impl BaseQueryOptions for Options {
    fn static_data(&self) -> &BaseQueryOptionsStatic {
        &self.statics
    }

    fn has_measures(&self) -> Result<bool, CubeError> {
        Ok(true)
    }

    fn measures(&self) -> Result<Option<Vec<OptionsMember>>, CubeError> {
        Ok(Some(self.measures.clone()))
    }

    fn has_dimensions(&self) -> Result<bool, CubeError> {
        Ok(true)
    }

    fn dimensions(&self) -> Result<Option<Vec<OptionsMember>>, CubeError> {
        Ok(Some(self.dimensions.clone()))
    }

    fn has_segments(&self) -> Result<bool, CubeError> {
        Ok(!self.segments.is_empty())
    }

    fn segments(&self) -> Result<Option<Vec<OptionsMember>>, CubeError> {
        Ok(Some(self.segments.clone()).filter(|segments| !segments.is_empty()))
    }

    fn cube_evaluator(&self) -> Result<Rc<dyn CubeEvaluator>, CubeError> {
        Ok(self.evaluator.clone())
    }

    fn base_tools(&self) -> Result<Rc<dyn BaseTools>, CubeError> {
        Ok(self.tools.clone())
    }

    fn join_graph(&self) -> Result<Rc<dyn JoinGraph>, CubeError> {
        Ok(self.joins.clone())
    }

    fn security_context(&self) -> Result<Rc<dyn SecurityContext>, CubeError> {
        Ok(self.security_context.clone())
    }

    fn has_join_hints(&self) -> Result<bool, CubeError> {
        Ok(false)
    }

    fn join_hints(&self) -> Result<Option<Vec<JoinHintItem>>, CubeError> {
        Ok(None)
    }

    fn has_subquery_joins(&self) -> Result<bool, CubeError> {
        Ok(false)
    }

    fn subquery_joins(&self) -> Result<Option<Vec<Rc<dyn SubqueryJoin>>>, CubeError> {
        Ok(None)
    }

    fn as_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}
