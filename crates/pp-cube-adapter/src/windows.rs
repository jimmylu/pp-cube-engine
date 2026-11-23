//! Rolling windows and time shifts: what a query must give them, and what the
//! dialect can write.
//!
//! The planner writes both without these checks, as Cube does. A measure over a
//! window of time then compiles without any time to roll or shift along, to an
//! interval literal the engine does not have, or to a time series whose periods
//! do not start where the granularity's periods do.

use chrono::{Datelike, NaiveDate, Weekday};
use pp_cube_dialect::Dialect;
use pp_cube_model::{Cube, Filter, Granularity, Measure, Model, Query, Target};

use crate::PlanError;

/// A measure the query reads that rolls or shifts, with the cube it belongs to.
struct Windowed<'m> {
    cube: &'m Cube,
    measure: &'m Measure,
}

impl Windowed<'_> {
    fn name(&self) -> String {
        format!("{}.{}", self.cube.name, self.measure.name)
    }

    /// The intervals the planner renders for this measure.
    fn intervals(&self) -> impl Iterator<Item = &str> {
        let window = self.measure.rolling_window.iter();
        let bounds = window.flat_map(|window| [&window.trailing, &window.leading]);
        let bounds = bounds.flatten().map(String::as_str);
        let shifts = self.measure.time_shift.iter();
        bounds
            .filter(|bound| *bound != "unbounded")
            .chain(shifts.map(|shift| shift.interval.as_str()))
    }
}

fn filter_members<'q>(filters: &'q [Filter], members: &mut Vec<&'q str>) {
    for filter in filters {
        match filter {
            Filter::And(filters) | Filter::Or(filters) => filter_members(filters, members),
            Filter::Condition(condition) => members.push(&condition.member),
        }
    }
}

/// The rolling and shifted measures `query` names or computes others from.
fn windowed<'m>(model: &'m Model, query: &Query) -> Vec<Windowed<'m>> {
    let mut named: Vec<&str> = query.measures.iter().map(String::as_str).collect();
    filter_members(&query.filters, &mut named);
    let mut pending: Vec<(&Cube, &Measure)> = Vec::new();
    for member in named {
        let Some((cube, name)) = member.split_once('.') else {
            continue;
        };
        let Some(cube) = model.cube(cube) else {
            continue;
        };
        if let Some(measure) = cube.measures.iter().find(|measure| measure.name == name) {
            pending.push((cube, measure));
        }
    }
    let mut seen: Vec<(&str, &str)> = Vec::new();
    let mut found = Vec::new();
    while let Some((cube, measure)) = pending.pop() {
        if seen.contains(&(cube.name.as_str(), measure.name.as_str())) {
            continue;
        }
        seen.push((&cube.name, &measure.name));
        if measure.rolling_window.is_some() || !measure.time_shift.is_empty() {
            found.push(Windowed { cube, measure });
        }
        let conditions = measure.filters.iter().map(|filter| &filter.sql);
        for expression in measure.sql.iter().chain(conditions) {
            // An expression that does not resolve is reported by the checks before this one.
            for target in model.resolve(cube, expression).unwrap_or_default() {
                if let Target::Measure(owner, measure) = target {
                    pending.push((owner, measure));
                }
            }
        }
    }
    found.sort_by_key(Windowed::name);
    found
}

fn unit_of(granularity: &Granularity) -> Option<&'static str> {
    match granularity {
        Granularity::Day => Some("day"),
        Granularity::Week => Some("week"),
        Granularity::Month => Some("month"),
        Granularity::Quarter => Some("quarter"),
        Granularity::Year => Some("year"),
        Granularity::Second | Granularity::Minute | Granularity::Hour => None,
        Granularity::Custom(_) => None,
    }
}

/// Whether `text`, a date or a timestamp of the query, is the first instant of
/// a period of `granularity`.
fn starts_period(text: &str, granularity: &Granularity) -> bool {
    let (day, time) = text.split_at(text.len().min("YYYY-MM-DD".len()));
    let midnight = time
        .bytes()
        .all(|byte| matches!(byte, b'T' | b'0' | b':' | b'.'));
    let Ok(day) = NaiveDate::parse_from_str(day, "%Y-%m-%d") else {
        return false;
    };
    midnight
        && match granularity {
            Granularity::Week => day.weekday() == Weekday::Mon,
            Granularity::Month => day.day() == 1,
            Granularity::Quarter => day.day() == 1 && day.month0() % 3 == 0,
            Granularity::Year => day.ordinal() == 1,
            _ => true,
        }
}

/// Fails for a query whose rolling or shifted measures have no time dimension to
/// work along, or need SQL `dialect` cannot express.
pub(crate) fn check(model: &Model, query: &Query, dialect: &Dialect) -> Result<(), PlanError> {
    let measures = windowed(model, query);
    let Some(first) = measures.first() else {
        return Ok(());
    };
    if query.time_dimensions.is_empty() {
        return Err(PlanError::Unsupported(format!(
            "Measure `{}` rolls or shifts over time, and the query has no time dimension; \
             without one it would be computed over all time",
            first.name()
        )));
    }
    for measure in &measures {
        // A shift along a named time dimension leaves every other one as it is.
        for shift in &measure.measure.time_shift {
            let Some(dimension) = &shift.time_dimension else {
                continue;
            };
            let path = format!("{}.{dimension}", measure.cube.name);
            let mut queried = query.time_dimensions.iter();
            if !queried.any(|time_dimension| time_dimension.dimension == path) {
                return Err(PlanError::Unsupported(format!(
                    "Measure `{}` shifts along `{path}`, which is not a time dimension of the \
                     query; it would not be shifted",
                    measure.name()
                )));
            }
        }
        for interval in measure.intervals() {
            let unit = interval.split(' ').nth(1).unwrap_or_default();
            if !dialect.has_interval_unit(unit) {
                return Err(PlanError::Unsupported(format!(
                    "Measure `{}` uses the interval `{interval}`, and {} has no interval of \
                     that unit",
                    measure.name(),
                    dialect.name()
                )));
            }
        }
    }
    // A time series is built for the rolling windows of a query with a granularity.
    let rolling = measures.iter().find(|m| m.measure.rolling_window.is_some());
    let Some(rolling) = rolling else {
        return Ok(());
    };
    for time_dimension in &query.time_dimensions {
        let Some(granularity) = &time_dimension.granularity else {
            continue;
        };
        // The periods of a declared granularity are counted from an origin the
        // time series does not know.
        if let Granularity::Custom(name) = granularity {
            return Err(PlanError::Unsupported(format!(
                "Measure `{}` has a rolling window, and it cannot roll by `{name}`, a \
                 granularity `{}` declares",
                rolling.name(),
                time_dimension.dimension
            )));
        }
        if !dialect.generates_time_series() {
            // The periods are written out one by one, so their range has to be known.
            if time_dimension.date_range.is_none() {
                return Err(PlanError::Unsupported(format!(
                    "Measure `{}` has a rolling window, and on {} `{}` needs a date range \
                     to roll by its granularity",
                    rolling.name(),
                    dialect.name(),
                    time_dimension.dimension
                )));
            }
            continue;
        }
        let Some(unit) = unit_of(granularity) else {
            continue;
        };
        if !dialect.has_interval_unit(unit) {
            return Err(PlanError::Unsupported(format!(
                "Measure `{}` has a rolling window, and {} cannot generate a time series by \
                 {unit}",
                rolling.name(),
                dialect.name()
            )));
        }
        let start = time_dimension.date_range.as_ref().map(|range| &range.from);
        if start.is_some_and(|start| !starts_period(start.as_str(), granularity)) {
            return Err(PlanError::Unsupported(format!(
                "Measure `{}` has a rolling window, and on {} its time series starts at the \
                 start of the date range of `{}`, which is not the start of a {unit}",
                rolling.name(),
                dialect.name(),
                time_dimension.dimension
            )));
        }
    }
    Ok(())
}
