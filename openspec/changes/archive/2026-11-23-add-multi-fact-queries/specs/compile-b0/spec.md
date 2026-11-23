# Spec Delta

## MODIFIED Requirements

### Requirement: B0 boundary
The B0 feature matrix SHALL be: the six aggregate types, filtered measures, number formulas, joined dimensions, every contract-query filter operator, and/or groups, segments, absolute time ranges, the five granularities, timezone conversion, explicit order, limit and offset. The kernel SHALL return unsupported, without SQL, for query features of a higher level that are not implemented and for matrix members outside the target dialect's declared support.
#### Scenario: B0-AGG-003 Higher-level query
- **WHEN** a query with a valid B0 bundle requests ungrouped output, compareDateRange or a recognized Cube granularity outside B0
- **THEN** compilation returns unsupported without SQL

### Requirement: Measures of joined cubes
The measures of a query MAY belong to a cube reached through joins. Through one_to_one joins every B0 measure type SHALL compile in the same flat SELECT. When a many_to_one join lies on the way, count and count_distinct SHALL compile in the flat SELECT, count as a distinct count of the cube's primary key. Every other measure type through a many_to_one join, and measures of more than one cube, SHALL compile as compile-b1 states.
#### Scenario: B0-JOIN-006 Joined measures
- **WHEN** queries take sum, avg, count and a number formula from a cube joined one_to_one, and count and count_distinct from a cube joined many_to_one
- **THEN** each compiles to one flat SELECT, with the many_to_one count as a distinct count of the primary key
