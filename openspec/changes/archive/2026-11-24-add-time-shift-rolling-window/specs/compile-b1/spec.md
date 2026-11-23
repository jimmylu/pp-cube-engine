# Spec Delta

## ADDED Requirements

### Requirement: Rolling windows
A measure with a bounded rolling window SHALL be aggregated, for each period of the query's time dimension, over the rows whose time lies in that period's window, as the pinned planner writes it: a time series of the range joined to the measure's rows by the window's bounds. Rows the window needs outside the date range SHALL be read, and a period without rows SHALL be present with no value. Everything else of the query SHALL apply as it does without a window.
#### Scenario: B1-ROLL-001 Trailing and leading windows
- **WHEN** a seven-day trailing sum is requested by day beside the plain sum and a dimension, and a window around the start of each month and an unbounded trailing sum are requested by month
- **THEN** each SQL joins a time series of the range to the measure's source by the window's bounds, the source reads back before the range by the window's length, and the time column carries its member and granularity

### Requirement: Period to date
A measure with a `to_date` window SHALL be aggregated, for each period of the query's time dimension, from the start of the window's granularity that holds the period to the end of the period.
#### Scenario: B1-ROLL-002 Month and year to date
- **WHEN** a month-to-date sum is requested by day and a year-to-date sum by month beside the plain sum
- **THEN** each SQL joins the time series to rows from the truncated start of its period to the period's end

### Requirement: Window without a granularity
When the query's time dimension has a date range and no granularity, a windowed measure SHALL be one value: the window that ends at the end of the range.
#### Scenario: B1-ROLL-003 Single window
- **WHEN** a seven-day trailing sum and a month-to-date sum are requested over October without a granularity
- **THEN** each compiles without a time series, bounded to the seven days and to the month that end with the range

### Requirement: Windows over rows
An avg or count_distinct with a rolling window SHALL be computed over the rows of the window, not over per-period results.
#### Scenario: B1-ROLL-004 Average and distinct count
- **WHEN** a seven-day average and a seven-day distinct count are requested by day
- **THEN** the source keeps one row per fact row and the window applies the aggregate to them

### Requirement: Time shifts
A multi-stage number measure with a time shift SHALL be the measures it is computed from, read with their time moved by the interval: later for prior, earlier for next, along the time dimension it names or, when it names none, along the query's. It SHALL be combined with unshifted measures over the query's dimensions as measures of several cubes are.
#### Scenario: B1-SHIFT-001 Prior and next period
- **WHEN** a sum and the same sum a year prior are requested by month over a year, and a sum a month ahead is requested
- **THEN** the shifted aggregate adds or subtracts the interval on the time column in its grouping and its range, the range is bound once for each aggregate, and the two are joined on the month

### Requirement: Shifted measures in formulas and filters
A multi-stage number measure MAY be computed from shifted and unshifted measures. Dimension filters, segments and security filters SHALL restrict every period; a filter on a shifted measure SHALL restrict the combined rows whether or not the measure is selected.
#### Scenario: B1-SHIFT-002 Growth and filtered shift
- **WHEN** a growth formula over a sum and its prior-year value is requested, and a query filters on the prior-year value without selecting it, with a segment and a dimension filter
- **THEN** the formula is computed from the two combined aggregates, the segment and dimension filter appear in both, and the measure filter applies to the combined rows without adding a column

### Requirement: Windows and shifts need their time dimension
A query that reads a measure with a rolling window or a time shift, selected, filtered on or computed from, SHALL have a time dimension; a shift that names a time dimension SHALL find it among the query's. Otherwise the kernel SHALL return unsupported without SQL, naming the measure: the pinned planner would compute it over all time or leave it unshifted.
#### Scenario: B1-WIN-001 Missing time dimension
- **WHEN** queries without a time dimension select a windowed measure, a shifted measure or a formula over one, or filter on a windowed measure, and a query shifts along a time dimension other than the one its shift names
- **THEN** each returns unsupported naming the measure, on every dialect, while a shift that names no time dimension follows the query's
