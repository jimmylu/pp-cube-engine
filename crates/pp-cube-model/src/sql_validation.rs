//! Private AST checks. Only the parsing copy contains reference sentinels.
use std::{collections::BTreeSet, ops::ControlFlow};

use serde_norway::Value;
use sqlparser::{
    ast::{
        Expr, GroupByExpr, Query, SelectFlavor, SetExpr, Statement, TableFactor, Visit, Visitor,
    },
    dialect::GenericDialect,
    parser::Parser,
    tokenizer::Token,
};

use crate::validation::{issue, items, text, ValidationIssue};
use crate::{locate_references, Reference};

pub(super) fn validate(value: &Value, out: &mut Vec<ValidationIssue>) {
    let cubes = items(value, "cubes");
    for (i, cube) in cubes.iter().enumerate() {
        let base = format!("/cubes/{i}");
        if let Some(sql) = text(cube, "sql") {
            let at = format!("{base}/sql");
            match Parser::parse_sql(&GenericDialect {}, sql) {
                Err(_) => issue(out, &at, "sql-syntax", "SQL could not be parsed"),
                Ok(statements) => {
                    let allowed =
                        matches!(statements.as_slice(), [Statement::Query(q)] if row_query(q));
                    let mut scan = Scan::default();
                    let _ = statements.visit(&mut scan);
                    if !allowed || scan.aggregate || scan.window || scan.queries > 1 {
                        issue(
                            out,
                            &at,
                            "sql-construct",
                            "Expected one single-table row-level SELECT",
                        );
                    }
                }
            }
        }
        for group in ["dimensions", "measures", "segments", "joins"] {
            for (j, member) in items(cube, group).iter().enumerate() {
                let at = format!("{base}/{group}/{j}/sql");
                check(member, cube, cubes, &at, group == "measures", out);
                for (k, filter) in items(member, "filters").iter().enumerate() {
                    check(
                        filter,
                        cube,
                        cubes,
                        &format!("{base}/{group}/{j}/filters/{k}/sql"),
                        false,
                        out,
                    );
                }
            }
        }
    }
}

fn row_query(q: &Query) -> bool {
    let Query {
        with,
        body,
        order_by,
        limit_clause,
        fetch,
        locks,
        for_clause,
        settings,
        format_clause,
        pipe_operators,
    } = q;
    if with.is_some()
        || order_by.is_some()
        || limit_clause.is_some()
        || fetch.is_some()
        || !locks.is_empty()
        || for_clause.is_some()
        || settings.is_some()
        || format_clause.is_some()
        || !pipe_operators.is_empty()
    {
        return false;
    }
    let SetExpr::Select(s) = body.as_ref() else {
        return false;
    };
    // All non-row clauses are rejected explicitly. The pinned AST makes this boundary stable.
    if s.distinct.is_some()
        || s.top.is_some()
        || s.select_modifiers.is_some()
        || s.exclude.is_some()
        || s.into.is_some()
        || !s.lateral_views.is_empty()
        || s.prewhere.is_some()
        || !s.connect_by.is_empty()
        || !s.cluster_by.is_empty()
        || !s.distribute_by.is_empty()
        || !s.sort_by.is_empty()
        || s.having.is_some()
        || !s.named_window.is_empty()
        || s.qualify.is_some()
        || s.value_table_mode.is_some()
        || s.flavor != SelectFlavor::Standard
        || !matches!(&s.group_by, GroupByExpr::Expressions(e, m) if e.is_empty() && m.is_empty())
    {
        return false;
    }
    let [table] = s.from.as_slice() else {
        return false;
    };
    table.joins.is_empty()
        && matches!(&table.relation,
        TableFactor::Table { args: None, with_hints, version: None, with_ordinality: false,
            partitions, json_path: None, sample: None, index_directives, inheritance: None, .. }
        if with_hints.is_empty() && partitions.is_empty() && index_directives.is_empty())
}

#[derive(Default)]
struct Scan {
    aggregate: bool,
    window: bool,
    queries: usize,
    qualifiers: Vec<Vec<String>>,
}
impl Visitor for Scan {
    type Break = ();
    fn pre_visit_query(&mut self, _: &Query) -> ControlFlow<()> {
        self.queries += 1;
        ControlFlow::Continue(())
    }
    fn pre_visit_expr(&mut self, e: &Expr) -> ControlFlow<()> {
        match e {
            Expr::Function(f) => {
                let name = f.name.0.last().map(ToString::to_string).unwrap_or_default();
                self.aggregate |=
                    aggregate(name.trim_matches(['"', '`']).to_ascii_lowercase().as_str())
                        || f.filter.is_some()
                        || !f.within_group.is_empty();
                self.window |= f.over.is_some();
            }
            Expr::CompoundIdentifier(parts) => self
                .qualifiers
                .push(parts.iter().map(|p| p.value.clone()).collect()),
            _ => {}
        }
        ControlFlow::Continue(())
    }
}

// Built-in classification, not a claim about opaque UDF implementations.
// Sources: Presto aggregate functions, Doris aggregate functions, Druid SQL aggregations.
fn aggregate(name: &str) -> bool {
    matches!(
        name,
        "approx_most_frequent"
            | "bitwise_xor_agg"
            | "classification_fall_out"
            | "classification_miss_rate"
            | "classification_precision"
            | "classification_recall"
            | "classification_thresholds"
            | "differential_entropy"
            | "khyperloglog_agg"
            | "qdigest_agg"
            | "regr_avgx"
            | "regr_avgy"
            | "regr_count"
            | "regr_intercept"
            | "regr_r2"
            | "regr_slope"
            | "regr_sxx"
            | "regr_sxy"
            | "regr_syy"
            | "reservoir_sample"
            | "approx_count_distinct_builtin"
            | "approx_count_distinct_ds_hll"
            | "approx_count_distinct_ds_theta"
            | "approx_quantile"
            | "approx_quantile_ds"
            | "approx_quantile_fixed_buckets"
            | "bit_and"
            | "bit_or"
            | "bit_xor"
            | "bloom_filter"
            | "ds_quantiles_sketch"
            | "ds_tuple_doubles"
            | "grouping"
            | "spectator_count"
            | "spectator_percentile"
            | "tdigest_generate_sketch"
            | "tdigest_quantile"
            | "ai_agg"
            | "avg_map"
            | "avg_weighted"
            | "bitmap_agg"
            | "bitmap_intersect"
            | "bitmap_union_int"
            | "bool_xor"
            | "corr_welford"
            | "count_by_enum"
            | "count_map"
            | "covar"
            | "datasketches_hll_union_agg"
            | "exponential_moving_average"
            | "group_array_intersect"
            | "group_array_union"
            | "group_bit_and"
            | "group_bit_or"
            | "group_bit_xor"
            | "group_bitmap_xor"
            | "intersect_count"
            | "kurt"
            | "linear_histogram"
            | "max_map"
            | "median"
            | "min_map"
            | "percentile_approx_array"
            | "percentile_approx_weighted"
            | "percentile_array"
            | "percentile_reservoir"
            | "quantile_union"
            | "retention"
            | "sem"
            | "sequence_count"
            | "sequence_match"
            | "skew"
            | "sum0"
            | "sum_map"
            | "topn"
            | "topn_array"
            | "topn_weighted"
            | "window_funnel"
            | "count"
            | "sum"
            | "avg"
            | "min"
            | "max"
            | "any_value"
            | "arbitrary"
            | "array_agg"
            | "array_concat_agg"
            | "bool_and"
            | "bool_or"
            | "every"
            | "count_if"
            | "bitwise_and_agg"
            | "bitwise_or_agg"
            | "checksum"
            | "geometric_mean"
            | "max_by"
            | "min_by"
            | "map_agg"
            | "map_union"
            | "map_union_sum"
            | "multimap_agg"
            | "reduce_agg"
            | "set_agg"
            | "set_union"
            | "approx_distinct"
            | "approx_percentile"
            | "approx_set"
            | "merge"
            | "numeric_histogram"
            | "histogram"
            | "entropy"
            | "corr"
            | "covar_pop"
            | "covar_samp"
            | "stddev"
            | "stddev_pop"
            | "stddev_samp"
            | "variance"
            | "var_pop"
            | "var_samp"
            | "kurtosis"
            | "skewness"
            | "group_concat"
            | "string_agg"
            | "listagg"
            | "collect_list"
            | "collect_set"
            | "bitmap_union"
            | "bitmap_union_count"
            | "hll_union"
            | "hll_union_agg"
            | "hll_raw_agg"
            | "approx_count_distinct"
            | "ndv"
            | "percentile"
            | "percentile_approx"
            | "earliest"
            | "latest"
            | "earliest_by"
            | "latest_by"
            | "ds_hll"
            | "ds_theta"
    )
}

type ParsedExpression<'a> = (Expr, Vec<Reference<'a>>, String);

fn parsed(sql: &str) -> Option<Result<ParsedExpression<'_>, ()>> {
    let refs = locate_references(sql).ok()?;
    let mut prefix = "__pp_ref_".to_owned();
    while sql.to_ascii_lowercase().contains(&prefix) {
        prefix.push('_');
    }
    let mut copy = String::new();
    let mut end = 0;
    for (i, r) in refs.iter().enumerate() {
        copy.push_str(&sql[end..r.braces.start]);
        copy.push_str(&format!("{prefix}{i}"));
        end = r.braces.end;
    }
    copy.push_str(&sql[end..]);
    let result = (|| {
        let mut parser = Parser::new(&GenericDialect {})
            .try_with_sql(&copy)
            .map_err(|_| ())?;
        let expr = parser.parse_expr().map_err(|_| ())?;
        if parser.peek_token().token != Token::EOF {
            return Err(());
        }
        Ok((
            expr,
            refs.into_iter().map(|r| r.reference).collect(),
            prefix,
        ))
    })();
    Some(result)
}

fn check(
    v: &Value,
    own: &Value,
    cubes: &[Value],
    at: &str,
    measure: bool,
    out: &mut Vec<ValidationIssue>,
) {
    let Some(sql) = text(v, "sql") else {
        return;
    };
    if out.iter().any(|i| i.path == at && i.rule_id == "reference") {
        return;
    }
    let Some(result) = parsed(sql) else {
        return;
    };
    let Ok((expr, refs, prefix)) = result else {
        issue(
            out,
            at,
            "sql-syntax",
            "Expected one complete SQL expression",
        );
        return;
    };
    let mut scan = Scan::default();
    let _ = expr.visit(&mut scan);
    if scan.queries > 0 || scan.window {
        issue(
            out,
            at,
            "sql-construct",
            "Subqueries and windows are outside baseline expressions",
        );
    }
    if !measure
        && (scan.aggregate
            || refs
                .iter()
                .any(|r| reaches_aggregate(*r, own, cubes, &mut BTreeSet::new())))
    {
        issue(
            out,
            at,
            "expression-aggregate",
            "Row-level expressions cannot depend on aggregation",
        );
    }
    if scan.qualifiers.iter().any(|parts| {
        let qualifier = &parts[..parts.len() - 1];
        let sentinel = qualifier.len() == 1
            && qualifier[0]
                .strip_prefix(&prefix)
                .and_then(|n| n.parse::<usize>().ok())
                .and_then(|i| refs.get(i))
                .is_some_and(|r| matches!(r, Reference::Column(_)));
        let joined = qualifier.join(".");
        !sentinel
            && ![
                text(own, "name"),
                text(own, "sql_alias"),
                text(own, "sql_table"),
            ]
            .into_iter()
            .flatten()
            .any(|s| s.eq_ignore_ascii_case(&joined))
    }) {
        issue(
            out,
            at,
            "expression-column",
            "Physical columns must belong to the owning cube",
        );
    }
}

fn reaches_aggregate<'a>(
    r: Reference<'a>,
    own: &'a Value,
    cubes: &'a [Value],
    seen: &mut BTreeSet<(usize, String)>,
) -> bool {
    let (cube, name) = match r {
        Reference::Column(_) | Reference::Joined { .. } => return false,
        Reference::OwnMember(name) => (own, name),
        Reference::Member { cube, member } => {
            let Some(cube) = cubes.iter().find(|c| text(c, "name") == Some(cube)) else {
                return false;
            };
            (cube, member)
        }
    };
    let index = cubes
        .iter()
        .position(|c| std::ptr::eq(c, cube))
        .unwrap_or(usize::MAX);
    if !seen.insert((index, name.to_owned())) {
        return false;
    }
    for group in ["dimensions", "measures"] {
        if let Some(member) = items(cube, group)
            .iter()
            .find(|m| text(m, "name") == Some(name))
        {
            if group == "measures" && text(member, "type") != Some("number") {
                return true;
            }
            if let Some(Ok((expr, refs, _))) = text(member, "sql").and_then(parsed) {
                let mut scan = Scan::default();
                let _ = expr.visit(&mut scan);
                return scan.aggregate
                    || refs
                        .into_iter()
                        .any(|r| reaches_aggregate(r, cube, cubes, seen));
            }
        }
    }
    false
}
