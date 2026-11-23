# compile-b0 Specification

## Purpose
Specify the first supported query level without promising B1 behavior.

## Requirements

### Requirement: B0 boundary
The B0 feature matrix SHALL be: the six aggregate types, filtered measures, number formulas, joined dimensions, every contract-query filter operator, and/or groups, segments, absolute time ranges, the five granularities, timezone conversion, explicit order, limit and offset. The kernel SHALL return unsupported, without SQL, for B1 query features and for matrix members outside the target dialect's declared support.
#### Scenario: B0-AGG-003 Higher-level query
- **WHEN** a query with a valid B0 bundle requests either measures from more than one cube in an otherwise valid query connected by a unique usable join tree on one data source, or ungrouped output, compareDateRange or a recognized Cube granularity outside B0
- **THEN** compilation returns unsupported without SQL
- **AND** when multi-cube measures are the only requested feature outside B0, query validation takes precedence: invalid members or security filters, disconnected cubes, ambiguous join paths and differing data sources return a query error without SQL

### Requirement: Single-cube aggregation
The kernel SHALL compile count, count_distinct, sum, avg, min, max, filtered measures and number formulas using pinned planner semantics. Direct-table B0 queries SHALL use a flat SELECT; a cube defined by row-filtered sql MAY add its source subquery.
#### Scenario: B0-AGG-001 Aggregate families
- **WHEN** each of the six aggregate types is requested over sales with a grouping dimension
- **THEN** SQL groups by that dimension and applies the matching aggregate function to the measure expression
#### Scenario: B0-AGG-002 Conditional measures and ratios
- **WHEN** sales includes a filtered sum and a number formula dividing sum by NULLIF(distinct count, 0)
- **THEN** SQL applies the measure condition inside aggregation and divides aggregated results rather than row values

### Requirement: Dimension joins
The kernel SHALL compile many_to_one and one_to_one joins as LEFT JOINs, starting from the one cube that reaches every other cube of the query. It SHALL return a query error, without choosing, when the requested cubes have more than one eligible join path or belong to more than one data source, and when no join path connects the cubes.
#### Scenario: B0-JOIN-001 Joined dimension
- **WHEN** sales is grouped and filtered by a uniquely joined store dimension
- **THEN** SQL has one LEFT JOIN on the declared condition and aggregates sales in the same flat SELECT
#### Scenario: B0-JOIN-002 Ambiguous path
- **WHEN** the requested cubes are connected by more than one eligible join path
- **THEN** compilation returns a query error naming the cubes rather than selecting a path
#### Scenario: B0-JOIN-003 Cubes on different data sources
- **WHEN** a query uses members of cubes whose data_source values differ
- **THEN** compilation returns a query error without SQL
#### Scenario: B0-JOIN-004 Cubes without a usable join
- **WHEN** a query names cubes that no join path connects, in members or in a filter
- **THEN** compilation returns a query error without SQL, naming the cubes

### Requirement: Join conditions
A join condition MAY read cubes other than the two it joins, directly or through a member whose expression reads them. The kernel SHALL add those cubes to the query's joins and join them before the join that reads them, as Cube does. It SHALL return a query error without SQL when no such order exists. A query that does not need the join SHALL NOT be given its cubes.
#### Scenario: B0-JOIN-005 Join condition reading another cube
- **WHEN** sales joins stores on a condition that reads regions, which is reached only through stores, and one query groups sales by a store dimension while another reads sales alone
- **THEN** the first returns a query error naming the joins that cannot be ordered, and the second compiles without a join
#### Scenario: B0-JOIN-007 Third cube joined first
- **WHEN** sales joins promotions on a condition that reads regions, reached through stores, and a query groups sales by a promotion dimension with and without naming a region dimension
- **THEN** SQL joins stores, then regions, then promotions, in that order in both queries, in one flat SELECT

### Requirement: Measures of joined cubes
The measures of a query SHALL belong to one cube, which MAY be reached through joins. Through one_to_one joins every B0 measure type SHALL compile in the same flat SELECT. When a many_to_one join lies on the way, count and count_distinct SHALL compile, count as a distinct count of the cube's primary key, and every other measure type SHALL return unsupported without SQL, naming the measure. A compilation the planner can only express as multi-stage SQL SHALL return unsupported.
#### Scenario: B0-JOIN-006 Joined measures
- **WHEN** queries take sum, avg, count and a number formula from a cube joined one_to_one, count and count_distinct from a cube joined many_to_one, and sum, avg, min and max from that cube
- **THEN** the first two compile to one flat SELECT each, with the many_to_one count as a distinct count of the primary key, and each of the last returns unsupported naming the measure

### Requirement: Speculative join lookups
When the planner asks whether a join tree exists for a set of cubes and none does, the kernel SHALL answer that there is none without failing the compilation. A lookup the kernel refuses, such as an ambiguous path, SHALL fail the compilation with its own message even if the planner goes on to succeed.
#### Scenario: B0-JOIN-008 Missing path and refusal
- **WHEN** the planner looks up cubes no join path connects, then cubes that are connected, then cubes with two join paths, then connected cubes again
- **THEN** the first lookup reports no tree and leaves the compilation able to succeed, and after the third the compilation fails with the ambiguity whatever follows
