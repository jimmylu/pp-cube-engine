//! A rolling window for an engine that joins on equality only.
//!
//! The planner writes a rolling window as the time series left-joined to the
//! aggregate on a range of time and grouped by period. An engine such as Druid
//! cannot join on a range. The same rows come from two steps it can run: the
//! series crossed with the aggregate, filtered by the range and grouped; then
//! the series left-joined to that result on the period, by equality, so a
//! period without rows still gives its row of NULLs.
//!
//! A dialect asks for this by having the template `kernel/rolling_select`. The
//! planner's SELECT is recognised by its join, `LEFT JOIN … AS "rolling_source"
//! ON …`, the alias the pinned planner gives the aggregate of a rolling window.

use cubenativeutils::CubeError;
use minijinja::{context, Value};
use pp_cube_dialect::Dialect;

pub(crate) const TEMPLATE: &str = "kernel/rolling_select";
const JOIN: &str = "LEFT JOIN ";
const SOURCE: &str = " AS \"rolling_source\" ON ";
const SERIES: &str = "\"time_series\".";

fn text(context: &Value, name: &str) -> Option<String> {
    let value = context.get_attr(name).ok()?;
    value.as_str().map(str::to_owned)
}

fn present(context: &Value, name: &str) -> bool {
    context
        .get_attr(name)
        .is_ok_and(|value| !value.is_undefined() && !value.is_none() && value.is_true())
}

/// The planner's rolling SELECT in the two-step form, or `None` when `context`
/// is not one. A rolling SELECT with a part this form has no place for fails:
/// the SQL as planned would not run on the engine.
pub(crate) fn select(dialect: &Dialect, context: &Value) -> Option<Result<String, CubeError>> {
    dialect.template(TEMPLATE)?;
    let from = text(context, "from_prepared")?;
    let join = from.find(JOIN)?;
    let on = from.find(SOURCE)?;
    if on < join {
        return None;
    }
    let series = from[..join].trim();
    let source = format!(
        "{} AS \"rolling_source\"",
        from[join + JOIN.len()..on].trim()
    );
    let condition = from[on + SOURCE.len()..].trim();
    let fail = |what: &str| {
        Some(Err(CubeError::internal(format!(
            "A rolling window SELECT has {what}, which `{TEMPLATE}` does not place"
        ))))
    };
    for part in ["ctes", "filter", "having", "distinct", "from"] {
        if present(context, part) {
            return fail(part);
        }
    }
    for part in ["limit", "offset"] {
        if context
            .get_attr(part)
            .is_ok_and(|v| !v.is_undefined() && !v.is_none())
        {
            return fail(part);
        }
    }
    let Some(group_by) = text(context, "group_by").filter(|g| !g.is_empty()) else {
        return fail("no GROUP BY");
    };
    let Ok(projection) = context.get_attr("select_concat") else {
        return fail("no projection");
    };
    let Ok(projection) = projection.try_iter() else {
        return fail("no projection");
    };
    let mut columns = Vec::new();
    for column in projection {
        let (Some(expr), Some(alias), Some(aliased)) = (
            text(&column, "expr"),
            text(&column, "alias"),
            text(&column, "aliased"),
        ) else {
            return fail("a column without a name");
        };
        // A period column is the series' own; every other comes from the aggregate.
        let period = expr.starts_with(SERIES);
        if !period && expr.contains(SERIES) {
            return fail("an expression over the series");
        }
        columns.push(context! { expr, alias, aliased, period });
    }
    if !columns
        .iter()
        .any(|c| c.get_attr("period").is_ok_and(|p| p.is_true()))
    {
        return fail("no period column");
    }
    let rendered = dialect.render(
        TEMPLATE,
        context! { columns, series, source, condition, group_by },
    );
    Some(rendered.map_err(|error| CubeError::internal(error.to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pp_cube_dialect::{dialect, DialectId};

    fn planned(extra: Value) -> Value {
        let column = |expr: &str, alias: &str| {
            context! { expr, alias, aliased => format!("{expr} \"{alias}\"") }
        };
        context! {
            select_concat => vec![
                column("\"time_series\".\"date_from\"", "day"),
                column("sum(\"rolling_source\".\"amount\")", "amount"),
            ],
            from_prepared => " time_series  AS \"time_series\"\nLEFT JOIN  cte_0  AS \"rolling_source\" ON \"rolling_source\".\"day\" <= \"time_series\".\"date_to\"",
            group_by => "1",
            ..extra
        }
    }

    // scenario: B1-DRUID-003
    #[test]
    fn only_the_planners_rolling_select_is_rewritten_and_only_whole() {
        let druid = dialect(DialectId::Druid).unwrap();
        let sql = select(druid, &planned(context! {})).unwrap().unwrap();
        assert!(
            sql.contains("INNER JOIN cte_0 AS \"rolling_source\" ON 1 = 1"),
            "{sql}"
        );
        assert!(
            sql.contains("WHERE \"rolling_source\".\"day\" <= \"time_series\".\"date_to\""),
            "{sql}"
        );
        assert!(
            sql.ends_with("AS \"rolling\" ON \"time_series\".\"date_from\" = \"rolling\".\"day\""),
            "{sql}"
        );
        // A dialect that joins on a range keeps the planner's SELECT.
        let presto = dialect(DialectId::Presto).unwrap();
        assert!(select(presto, &planned(context! {})).is_none());
        // Any other SELECT is left alone.
        let other = context! { from_prepared => "dw.sales AS \"sales\"", group_by => "1" };
        assert!(select(druid, &other).is_none());
        // A part the two-step form has no place for fails; it is never dropped.
        for extra in [
            context! { filter => "1 = 1" },
            context! { having => "1 = 1" },
            context! { limit => 5 },
            context! { ctes => vec!["x AS (SELECT 1)"] },
        ] {
            assert!(select(druid, &planned(extra)).unwrap().is_err());
        }
    }
}
