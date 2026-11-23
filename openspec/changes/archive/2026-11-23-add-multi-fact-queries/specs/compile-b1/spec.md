# Spec Delta

## ADDED Requirements

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
Every fact SHALL reach every cube the query's dimensions, time dimensions, filters and segments read, by one join path, and every cube of the query SHALL be on one data source whether or not a join connects them. Otherwise the kernel SHALL return a query error without SQL, naming the cubes. Model errors, unknown members and invalid security filters SHALL keep their precedence.
#### Scenario: B1-MULTI-005 Fact that cannot reach the query
- **WHEN** two facts are requested with a dimension, a time dimension or a filter of one of them only, with a dimension one of them reaches by two paths, or with one fact on another data source
- **THEN** each returns a query error without SQL, naming the cubes without a path or the ambiguity, and an invalid model, member or security filter is reported first

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
