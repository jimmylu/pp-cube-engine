//! A query in the form the planner takes.

use std::any::Any;
use std::rc::Rc;

use cubenativeutils::CubeError;
use cubesqlplanner::cube_bridge::base_query_options::{BaseQueryOptions, BaseQueryOptionsStatic};
use cubesqlplanner::cube_bridge::base_tools::BaseTools;
use cubesqlplanner::cube_bridge::evaluator::CubeEvaluator;
use cubesqlplanner::cube_bridge::join_graph::JoinGraph;
use cubesqlplanner::cube_bridge::join_hints::JoinHintItem;
use cubesqlplanner::cube_bridge::options_member::OptionsMember;
use cubesqlplanner::cube_bridge::security_context::SecurityContext;
use cubesqlplanner::cube_bridge::subquery_join::SubqueryJoin;
use pp_cube_model::Query;

pub(crate) struct Options {
    measures: Vec<OptionsMember>,
    dimensions: Vec<OptionsMember>,
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
        Self {
            measures: members(&query.measures),
            dimensions: members(&query.dimensions),
            evaluator,
            tools,
            joins,
            security_context,
            // Every field is named, so a field the planner adds shows up here
            // as a compile error when the pin moves.
            statics: BaseQueryOptionsStatic {
                time_dimensions: None,
                timezone: None,
                filters: None,
                order: None,
                limit: None,
                row_limit: None,
                offset: None,
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
        Ok(false)
    }

    fn segments(&self) -> Result<Option<Vec<OptionsMember>>, CubeError> {
        Ok(None)
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
