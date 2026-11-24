# dialect-druid Specification

## Purpose
Define what the Druid dialect compiles, declares and refuses.

## Requirements

### Requirement: Druid B0 compilation
The Druid dialect SHALL compile every member of the compile-b0 feature matrix, joined dimensions included, to the SQL of Cube's Druid dialect at the pinned commit, with the same ordered parameters, columns and lineage as the other dialects.
#### Scenario: B0-DRUID-001 Golden SQL
- **WHEN** each shared execution case the Druid dialect compiles, including each granularity with a non-UTC timezone and joins, is compiled for Druid
- **THEN** the SQL and ordered parameters equal the reviewed golden, on Druid's own schema, with parameters, columns and lineage equal to Presto's for the same query

### Requirement: Druid support declaration
The Druid dialect SHALL declare the query features its engine cannot answer: SQL of more than one SELECT, ungrouped queries, rolling windows, time shifts, approximate distinct counts and declared granularities. A query that uses one SHALL return unsupported without SQL, naming the feature and what uses it; it SHALL never fail as an internal error. Query errors SHALL keep their class.
#### Scenario: B0-DRUID-002 Declared refusals
- **WHEN** Druid queries take measures of two cubes, a sum over rows a join repeats, ungrouped rows, a rolling and a to-date measure, a formula over a shifted measure, a formula over an approximate count and a declared granularity, and others name an unknown member or cubes with two join paths
- **THEN** each of the first returns unsupported naming the feature and the measure or dimension, the last two return query errors, and every shared execution case is either compiled or refused this way

### Requirement: Druid query context
Every Druid compilation SHALL declare the query context `useApproximateCountDistinct` as false among its session properties, so that a distinct count is exact.
#### Scenario: B0-DRUID-005 Exact distinct counts
- **WHEN** a distinct count and a paged sum are compiled for Druid, and a sum for Presto
- **THEN** both Druid results declare the context and the Presto result declares nothing
