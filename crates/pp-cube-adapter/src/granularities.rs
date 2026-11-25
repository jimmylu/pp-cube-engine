//! Granularities a time dimension declares: where the SQL counts periods from
//! an origin, the rows have to lie at or after it.
//!
//! The planner counts such periods as whole units since the origin, which
//! truncates toward the origin: a row before it lands a period too late, in
//! Cube as here. A date range that starts at the origin or later leaves no
//! such row.

use pp_cube_dialect::Dialect;
use pp_cube_model::{CustomGranularity, Granularity, Model, Query};

use crate::PlanError;

/// Whether the planner counts the periods of `granularity` from its origin
/// rather than truncating to a natural period and moving it: an interval of
/// several units, or an origin on a day not every month has.
fn counted_from_origin(granularity: &CustomGranularity) -> Option<&str> {
    let origin = granularity.origin.as_deref()?;
    let day: u32 = origin.get(8..10)?.parse().ok()?;
    (!granularity.interval.starts_with("1 ") || day > 28).then_some(origin)
}

/// A date or a timestamp of the model or the query as a full local timestamp.
fn instant(text: &str) -> String {
    const FULL: &str = "0000-00-00T00:00:00.000";
    format!("{text}{}", FULL.get(text.len()..).unwrap_or_default())
}

/// Fails for a time dimension grouped by a granularity counted from an origin
/// when its date range can hold rows before that origin, and for one moved by
/// an offset in a unit the engine has no interval literal of.
pub(crate) fn check(model: &Model, query: &Query, dialect: &Dialect) -> Result<(), PlanError> {
    for time_dimension in &query.time_dimensions {
        let Some(Granularity::Custom(name)) = &time_dimension.granularity else {
            continue;
        };
        let member = &time_dimension.dimension;
        let declared = member.split_once('.').and_then(|(cube, dimension)| {
            let dimensions = &model.cube(cube)?.dimensions;
            let dimension = dimensions.iter().find(|d| d.name == dimension)?;
            dimension.granularities.iter().find(|g| g.name == *name)
        });
        // The offset is added and subtracted as written, one unit at a time.
        let offset = declared.and_then(|granularity| granularity.offset.as_deref());
        for unit in offset
            .into_iter()
            .flat_map(|o| o.split_whitespace().skip(1).step_by(2))
        {
            let unit = unit.to_lowercase();
            let unit = unit.strip_suffix('s').unwrap_or(&unit);
            if !dialect.has_interval_unit(unit) {
                return Err(PlanError::Unsupported(format!(
                    "Granularity `{name}` of `{member}` is offset by `{}`, and {} has no \
                     interval of that unit",
                    offset.unwrap_or_default(),
                    dialect.name()
                )));
            }
        }
        // One the dimension does not declare is reported by the query checks.
        let Some(origin) = declared.and_then(counted_from_origin) else {
            continue;
        };
        let start = time_dimension.date_range.as_ref().map(|range| &range.from);
        if start.is_none_or(|start| instant(start.as_str()) < instant(origin)) {
            return Err(PlanError::Unsupported(format!(
                "Granularity `{name}` of `{member}` counts its periods from `{origin}`, and \
                 rows before that would be put in the wrong period; give `{member}` a date \
                 range that starts there or later"
            )));
        }
    }
    Ok(())
}
