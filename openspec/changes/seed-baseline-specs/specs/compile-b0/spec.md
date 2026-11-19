## Purpose
Specify the first supported query level without promising B1 behavior.
## ADDED Requirements
### Requirement: Single-cube aggregation
The kernel SHALL compile count, count_distinct, sum, avg, min, max, filtered measures and number formulas using pinned planner semantics. Direct-table B0 queries SHALL use a flat SELECT; a cube defined by row-filtered sql MAY add its source subquery.
#### Scenario: B0-AGG-001 Aggregate families
- **WHEN** each of the six aggregate types is requested over sales with a grouping dimension
- **THEN** SQL groups by that dimension and applies the matching aggregate function to the measure expression
#### Scenario: B0-AGG-002 Conditional measures and ratios
- **WHEN** sales includes a filtered sum and a number formula dividing sum by NULLIF(distinct count, 0)
- **THEN** SQL applies the measure condition inside aggregation and divides aggregated results rather than row values
### Requirement: Dimension joins
The kernel SHALL compile many_to_one and one_to_one joins as LEFT JOINs. It SHALL return a query error, without choosing, when the requested cubes have more than one eligible join path or belong to more than one data source.
#### Scenario: B0-JOIN-001 Joined dimension
- **WHEN** sales is grouped and filtered by a uniquely joined store dimension
- **THEN** SQL has one LEFT JOIN on the declared condition and aggregates sales in the same flat SELECT
#### Scenario: B0-JOIN-002 Ambiguous path
- **WHEN** the requested cubes are connected by more than one eligible join path
- **THEN** compilation returns a query error naming the cubes rather than selecting a path
#### Scenario: B0-JOIN-003 Cubes on different data sources
- **WHEN** a query uses members of cubes whose data_source values differ
- **THEN** compilation returns a query error without SQL
### Requirement: B0 boundary
The B0 feature matrix SHALL be: the six aggregate types, filtered measures, number formulas, joined dimensions, every contract-query filter operator, and/or groups, segments, absolute time ranges, the five granularities, timezone conversion, explicit order, limit and offset. The kernel SHALL return unsupported, without SQL, for B1 query features and for matrix members outside the target dialect's declared support.
#### Scenario: B0-AGG-003 Higher-level query
- **WHEN** a query with a valid B0 bundle requests measures from more than one cube, ungrouped output, compareDateRange or a recognized Cube granularity outside B0
- **THEN** compilation returns unsupported without SQL
