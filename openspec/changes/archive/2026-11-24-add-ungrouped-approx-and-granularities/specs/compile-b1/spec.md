# Spec Delta

## ADDED Requirements

### Requirement: Ungrouped queries
A query with `ungrouped: true` SHALL return the rows its dimensions and time dimensions select, without grouping, as the pinned planner writes them: a sum, avg, min or max as its expression, a count or distinct count as one where its expression is present, and a filtered measure as its expression where the filter holds. Joins, dimension filters, segments, security filters, order, limit and offset SHALL apply as in a grouped query.
#### Scenario: B1-DETAIL-001 Rows
- **WHEN** an ungrouped query selects a key with a sum, a count, a distinct count and a filtered sum, and another selects a joined dimension and a time dimension with a filter, an order and a page
- **THEN** each compiles to one SELECT without GROUP BY whose measures are row expressions, with one column per selected member, and `ungrouped: false` compiles as the grouped query

### Requirement: Ungrouped boundaries
An ungrouped query SHALL NOT be filtered on a measure: the kernel SHALL return a query error naming the filter member, as Cube does. Where the planner would aggregate a measure apart from the rows, for measures of several cubes, over rows a join repeats, rolling or shifted, the kernel SHALL return unsupported without SQL.
#### Scenario: B1-DETAIL-002 What rows cannot carry
- **WHEN** an ungrouped query filters on a measure, selects a sum of a cube reached through a many_to_one join, or selects a rolling measure
- **THEN** the first returns a query error naming the measure and the others return unsupported, none with SQL

### Requirement: Approximate distinct count
A measure of type `count_distinct_approx` SHALL compile to the engine's own estimate of the distinct count, wherever a measure can be used: selected, in a number formula, in a measure filter and beside measures of other cubes.
#### Scenario: B1-APPROX-001 Estimate
- **WHEN** an approximate distinct count is selected with a formula over it and a filter on it, and beside a sum over repeated rows
- **THEN** the SQL applies the engine's approximate-distinct function in each place, and the exact count_distinct is unchanged

### Requirement: Declared granularities
A time dimension grouped by a granularity it declares SHALL be truncated to that granularity's periods in the query's timezone, as the pinned planner writes it: periods of one unit as natural periods moved by the offset, or by the origin's distance from a natural boundary; other periods as whole intervals counted from the origin. The result column SHALL carry the granularity's name.
#### Scenario: B1-GRAN-001 Periods from an origin and moved periods
- **WHEN** a sum is grouped by a six-month and a two-week granularity with origins, by a year with an offset and by a month whose origin is the 15th
- **THEN** the first two count whole intervals from their origin, the last two truncate naturally around their shift, the column names its granularity, and in another timezone the origin is a local time

### Requirement: Granularity boundaries
A granularity the time dimension does not declare SHALL be a query error naming both. A rolling window SHALL NOT roll by a declared granularity, and a granularity whose periods are counted from an origin SHALL be used only with a date range that starts at the origin or later, because the pinned planner's count puts rows before the origin in the wrong period; both SHALL return unsupported without SQL, naming the granularity.
#### Scenario: B1-GRAN-002 Undeclared, rolling and before the origin
- **WHEN** queries name an undeclared granularity, roll a windowed measure by a declared one, and group by an origin-counted granularity without a date range or with one that starts before the origin
- **THEN** the first returns a query error, the others return unsupported naming the granularity, and granularities that only move natural periods compile without a date range
