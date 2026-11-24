# compile-b1 Specification

## Purpose
Specify the second query level as it is implemented, unit by unit: first, queries that need more than one aggregation.

## Requirements

### Requirement: Measures of several cubes
A query MAY take measures from more than one cube. The kernel SHALL aggregate each such cube's measures on their own over the dimensions of the query and combine the results on those dimensions, treating NULL keys as equal, so that a dimension value any fact has appears once. Without dimensions the result SHALL be one row holding every fact's total. The SQL SHALL be the pinned planner's: one statement with a common table expression per aggregation.
#### Scenario: B1-MULTI-001 Facts over a shared dimension
- **WHEN** sales and refunds amounts are requested by a store dimension both cubes join, and again without a dimension
- **THEN** the first SQL aggregates each cube in its own common table expression and joins both to the distinct union of their store values with NULL-safe equality, and the second places the two totals side by side

### Requirement: Filters, order and paging over several facts
Dimension filters, segments and security filters SHALL restrict every fact, each binding its parameters once per fact in SQL order. Measure filters SHALL restrict the combined rows and MAY name a measure the query does not select. Order, limit and offset SHALL apply to the combined rows.
#### Scenario: B1-MULTI-002 Filtered, ordered page
- **WHEN** two facts by a store dimension carry a segment, a dimension filter, a filter on one fact's measure, an order and a page, and separately a security filter on the store dimension
- **THEN** the segment and the dimension filter appear in both aggregations with the value bound twice before the measure filter's value, the measure filter and the page apply to the combined rows, and the security filter appears in both aggregations

### Requirement: Measures over repeated rows
A sum, avg, min or max of a cube reached through a many_to_one join SHALL be aggregated over the distinct keys of that cube for each group, as Cube does, alone or beside measures of other cubes. Count and count_distinct SHALL stay as compile-b0 states them.
#### Scenario: B1-MULTI-003 Sum through a many_to_one join
- **WHEN** a store's floor area is summed by a sales dimension, alone and beside the sales amount, and avg, min and max are requested the same way
- **THEN** each aggregates over a subquery of distinct store keys per group, the sales amount is aggregated apart and combined with it, and a count of stores beside the sales amount stays in one flat SELECT

### Requirement: Measures one SELECT can hold
Measures of cubes joined one_to_one SHALL be aggregated in one flat SELECT, and so SHALL a number formula over measures of such cubes.
#### Scenario: B1-MULTI-004 One_to_one measures and a formula over two cubes
- **WHEN** a query takes the sales amount beside the receipts tax, and another takes a receipts formula dividing its tax by the sales amount
- **THEN** each compiles to one flat SELECT that aggregates both cubes

### Requirement: Fact boundaries
Every fact SHALL reach every cube the query's dimensions, time dimensions, filters and segments read, by one join path, unless one join tree holds every cube the query reads; and every cube of the query SHALL be on one data source whether or not a join connects them. Otherwise the kernel SHALL return a query error without SQL, naming the cubes. Model errors, unknown members and invalid security filters SHALL keep their precedence.
#### Scenario: B1-MULTI-005 Fact that cannot reach the query
- **WHEN** two facts are requested with a dimension, a time dimension or a filter of one of them only, with a dimension one of them reaches by two paths, or with one fact on another data source
- **THEN** each returns a query error without SQL, naming the cubes without a path or the ambiguity, and an invalid model, member or security filter is reported first
#### Scenario: B1-MULTI-008 One tree over the whole query
- **WHEN** a measure of a cube that declares no join is requested beside a measure of the cube that joins it one_to_one, grouped by a dimension only that cube reaches, and again without any member of the joining cube
- **THEN** the first compiles as Cube writes it, one SELECT from the joining cube, and the second returns a query error naming the cubes without a path

### Requirement: Filters on counts over repeated rows
A filter on a count measure, or on a formula computed from one, SHALL return unsupported without SQL, naming the filter member and the count, when the query reaches the count's cube through a many_to_one join. The pinned planner compares the count of repeated rows there while selecting the count of distinct keys. A count of the cube the joins start from, a count through one_to_one joins, and a count_distinct SHALL compile.
#### Scenario: B1-MULTI-006 Filtered count through a many_to_one join
- **WHEN** a filter compares a store count in queries that reach stores from sales, with the count selected or not, and in queries that start from stores, count receipts joined one_to_one, or filter a distinct count
- **THEN** the first return unsupported naming the filter member and the count, and the others compile

### Requirement: Result metadata over several facts
The compile result of a query over several facts SHALL list one column per selected member in projection order, and SHALL report as used every cube any aggregation reads with its physical tables, including cubes read only in a subquery of distinct keys or only by a filter.
#### Scenario: B1-MULTI-007 Columns and lineage
- **WHEN** two facts are compiled by a store dimension, a sum over repeated rows is compiled, and a fact is named only by a measure filter
- **THEN** columns follow the projection, the used cubes are the facts and the dimension cube with their tables, and the fact named only by the filter is used without being a column

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

### Requirement: Queries on views
A query MAY name the members of a view wherever it names members: measures, dimensions, time dimensions, filters, security filters and order. The kernel SHALL compile them as the members the view takes, joined along the cubes of the model as any query is, as the pinned planner writes them: the result columns SHALL carry the view's names, and the used cubes SHALL be the cubes behind the view. View members and cube members MAY stand in one query.
#### Scenario: B1-VIEW-001 Members under the view's name
- **WHEN** a view's sum is grouped by its own and a prefixed joined dimension, and its measures are grouped by a dimension two joins away with a time dimension, a dimension filter, a measure filter, an order and a page
- **THEN** each SQL is the SQL of the same query on the cubes but for the column names, with equal parameters and used cubes, the columns name the view's members, and a view dimension compiles beside a cube measure

### Requirement: Views carry every capability of their members
A member taken by a view SHALL keep what it does in its cube: aggregation over repeated rows, rolling windows and time shifts along the view's time dimension, declared granularities, formulas, filtered measures and ungrouped rows.
#### Scenario: B1-VIEW-002 Capabilities through a view
- **WHEN** a view is queried for a sum over rows its join repeats, a rolling measure, a shifted measure with its base, a declared granularity, a formula with a filtered sum and a one_to_one measure, and ungrouped rows
- **THEN** each compiles to the SQL its members compile to, under the view's column names

### Requirement: View boundaries
A member the view does not take SHALL be a query error naming it. Every refusal that holds for the members behind a view SHALL hold through it, and a security filter on a view member SHALL restrict the cube behind it.
#### Scenario: B1-VIEW-003 Rules of the members behind
- **WHEN** queries name a member the view excludes or does not include, filter a view on a count over repeated rows, take a view's rolling measure without a time dimension, and carry a security filter on a view dimension two joins away
- **THEN** the first are query errors naming the member, the next two return unsupported naming the member behind the view, and the security filter is a condition on the joined cube
