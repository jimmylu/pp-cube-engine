# Spec Delta

## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: Druid support declaration
The Druid dialect SHALL declare the query features its engine cannot answer: SQL of more than one SELECT, ungrouped queries, rolling windows and time shifts. A query that uses one SHALL return unsupported without SQL, naming the feature and what uses it; it SHALL never fail as an internal error. Query errors SHALL keep their class.
#### Scenario: B0-DRUID-002 Declared refusals
- **WHEN** Druid queries take measures of two cubes, a sum over rows a join repeats, ungrouped rows, a rolling and a to-date measure and a formula over a shifted measure, and others name an unknown member or cubes with two join paths
- **THEN** each of the first returns unsupported naming the feature and the measure or dimension, the last two return query errors, and every shared execution case is either compiled or refused this way
