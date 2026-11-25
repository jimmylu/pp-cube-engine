# Spec Delta

## ADDED Requirements

### Requirement: Druid queries of several SELECT statements
The Druid dialect SHALL compile measures of several cubes over shared dimensions, sums over rows a join repeats, time shifts and formulas over them, to SQL that Druid 26 executes: aggregates merged by a FULL JOIN on equality and a GROUP BY of the merged keys, so that rows whose key is NULL merge. Parameters, columns and lineage SHALL equal the other dialects'. A shift in a unit Druid's interval literal lacks SHALL return unsupported.
#### Scenario: B1-DRUID-002 Several SELECT statements on Druid
- **WHEN** Druid queries take measures of two cubes over a dimension, the same with a measure filter, an order and a page, a sum over repeated rows, a formula over a measure shifted by a year and totals of two cubes, and one a shift by a week
- **THEN** each of the first compiles to the reviewed golden without a null-safe join condition or a union, with parameters, columns and lineage equal to Presto's, and the last returns unsupported naming the interval

## MODIFIED Requirements

### Requirement: Druid support declaration
The Druid dialect SHALL declare the query features its engine cannot answer: ungrouped queries and rolling windows. A query that uses one SHALL return unsupported without SQL, naming the feature and what uses it; it SHALL never fail as an internal error. Query errors SHALL keep their class.
#### Scenario: B0-DRUID-002 Declared refusals
- **WHEN** Druid queries take ungrouped rows, a rolling measure and a to-date measure, and others name an unknown member or cubes with two join paths
- **THEN** each of the first returns unsupported naming the feature and the measure or dimension, the last two return query errors, and every shared execution case is either compiled or refused this way
