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
The Druid dialect SHALL declare the query features its engine cannot answer: ungrouped queries. A query that uses one SHALL return unsupported without SQL, naming the feature; it SHALL never fail as an internal error. Query errors SHALL keep their class.
#### Scenario: B0-DRUID-002 Declared refusals
- **WHEN** a Druid query takes ungrouped rows, and others name an unknown member or cubes with two join paths
- **THEN** the first returns unsupported naming the feature, the last two return query errors, and every shared execution case is either compiled or refused this way

### Requirement: Druid query context
Every Druid compilation SHALL declare the query context `useApproximateCountDistinct` as false among its session properties, so that a distinct count is exact.
#### Scenario: B0-DRUID-005 Exact distinct counts
- **WHEN** a distinct count and a paged sum are compiled for Druid, and a sum for Presto
- **THEN** both Druid results declare the context and the Presto result declares nothing

### Requirement: Druid approximate distinct counts and declared granularities
The Druid dialect SHALL compile approximate distinct counts and declared granularities to one SELECT that Druid 26 executes, with the parameters, columns and lineage of the other dialects. An offset in a unit Druid's interval literal lacks SHALL return unsupported.
#### Scenario: B1-DRUID-001 One SELECT beyond Cube Core
- **WHEN** Druid queries take a formula over an approximate count, granularities of six months, two weeks and one month counted from an origin, and a year offset by three months, and one a granularity offset by a week
- **THEN** each of the first compiles to the reviewed golden, with parameters and columns equal to Presto's for the same query, and the last returns unsupported naming the interval

### Requirement: Druid acceptance
Druid output SHALL be accepted by execution on a real Druid and by comparison with pinned Cube Core's Druid dialect. A Druid case SHALL compile to Cube Core's SQL and parameters, but for the default row limit, unless it states that Cube Core refuses the query or writes SQL that Druid rejects; a stated relation SHALL be verified against Cube Core and Druid.
#### Scenario: B0-DRUID-003 Cube Core comparison (T1)
- **WHEN** each Druid execution case is put to pinned Cube Core with its Druid dialect
- **THEN** its SQL and parameters equal the kernel's but for the default row limit, or Cube Core refuses it as the case states, or its different SQL is rejected by Druid as the case states
#### Scenario: B0-DRUID-004 Engine execution (T2)
- **WHEN** each Druid execution case runs on a real Druid engine loaded with its fixture and the declared query context
- **THEN** it executes and its column names and ordered rows equal the independently chosen expected result

### Requirement: Druid queries of several SELECT statements
The Druid dialect SHALL compile measures of several cubes over shared dimensions, sums over rows a join repeats, time shifts and formulas over them, to SQL that Druid 26 executes: aggregates merged by a FULL JOIN on equality and a GROUP BY of the merged keys, so that rows whose key is NULL merge. Parameters, columns and lineage SHALL equal the other dialects'. A shift in a unit Druid's interval literal lacks SHALL return unsupported.
#### Scenario: B1-DRUID-002 Several SELECT statements on Druid
- **WHEN** Druid queries take measures of two cubes over a dimension, the same with a measure filter, an order and a page, a sum over repeated rows, a formula over a measure shifted by a year and totals of two cubes, and one a shift by a week
- **THEN** each of the first compiles to the reviewed golden without a null-safe join condition or a union, with parameters, columns and lineage equal to Presto's, and the last returns unsupported naming the interval

### Requirement: Druid rolling windows
The Druid dialect SHALL compile rolling windows, period to date and running totals by a time granularity over a date range, to SQL Druid 26 executes with the rows of the other engines: no join on a range of time, and a period without rows keeps a row of NULLs. Rolling by a granularity without a date range SHALL return unsupported. A rolling SELECT that cannot be written this way SHALL fail rather than lose a part.
#### Scenario: B1-DRUID-003 Rolling windows on Druid
- **WHEN** Druid queries take a trailing window, a month to date and a trailing window by another dimension, each by day over a date range, one the trailing window without a date range, and a rolling SELECT is given a filter, a HAVING, a limit or CTEs of its own
- **THEN** the first three compile to the reviewed golden without a join on a range, with parameters, columns and lineage equal to Doris's, the fourth returns unsupported naming the measure and the time dimension, and the last fails without SQL
