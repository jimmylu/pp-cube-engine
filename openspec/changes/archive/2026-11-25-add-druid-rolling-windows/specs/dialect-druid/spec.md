# Spec Delta

## ADDED Requirements

### Requirement: Druid rolling windows
The Druid dialect SHALL compile rolling windows, period to date and running totals by a time granularity over a date range, to SQL Druid 26 executes with the rows of the other engines: no join on a range of time, and a period without rows keeps a row of NULLs. Rolling by a granularity without a date range SHALL return unsupported. A rolling SELECT that cannot be written this way SHALL fail rather than lose a part.
#### Scenario: B1-DRUID-003 Rolling windows on Druid
- **WHEN** Druid queries take a trailing window, a month to date and a trailing window by another dimension, each by day over a date range, one the trailing window without a date range, and a rolling SELECT is given a filter, a HAVING, a limit or CTEs of its own
- **THEN** the first three compile to the reviewed golden without a join on a range, with parameters, columns and lineage equal to Doris's, the fourth returns unsupported naming the measure and the time dimension, and the last fails without SQL

## MODIFIED Requirements

### Requirement: Druid support declaration
The Druid dialect SHALL declare the query features its engine cannot answer: ungrouped queries. A query that uses one SHALL return unsupported without SQL, naming the feature; it SHALL never fail as an internal error. Query errors SHALL keep their class.
#### Scenario: B0-DRUID-002 Declared refusals
- **WHEN** a Druid query takes ungrouped rows, and others name an unknown member or cubes with two join paths
- **THEN** the first returns unsupported naming the feature, the last two return query errors, and every shared execution case is either compiled or refused this way
