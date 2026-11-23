# dialect-presto Specification

## Purpose
Define the Presto output and acceptance boundary for baseline queries.

## Requirements

### Requirement: Presto B0 compilation
The Presto dialect SHALL compile every member of the compile-b0 feature matrix to Presto SQL.
#### Scenario: B0-PRESTO-001 Non-UTC granularities
- **WHEN** day, week, month, quarter and year are each compiled for Presto with Asia/Tokyo and an absolute October date range
- **THEN** every SQL and ordered parameter result equals its reviewed golden snapshot
#### Scenario: B0-PRESTO-004 Aggregate families
- **WHEN** all six aggregate types are requested with a grouping dimension for Presto
- **THEN** the SQL and ordered parameters equal their reviewed golden snapshot
#### Scenario: B0-PRESTO-005 Conditional measures and ratios
- **WHEN** a filtered sum and a number formula over aggregates are requested for Presto
- **THEN** the SQL and ordered parameters equal their reviewed golden snapshot
#### Scenario: B0-PRESTO-006 Joined dimension
- **WHEN** sales is grouped and filtered by a uniquely joined store dimension for Presto
- **THEN** the SQL and ordered parameters equal their reviewed golden snapshot
#### Scenario: B0-PRESTO-007 Equality and presence
- **WHEN** equals, notEquals, set and notSet are exercised on compatible members for Presto
- **THEN** the SQL and ordered parameters equal their reviewed golden snapshot
#### Scenario: B0-PRESTO-008 String filters
- **WHEN** contains, notContains, startsWith, notStartsWith, endsWith and notEndsWith are exercised with pattern characters for Presto
- **THEN** the SQL and ordered parameters equal their reviewed golden snapshot
#### Scenario: B0-PRESTO-009 Number comparisons
- **WHEN** gt, gte, lt and lte are exercised on a number dimension for Presto
- **THEN** the SQL and ordered parameters equal their reviewed golden snapshot
#### Scenario: B0-PRESTO-010 Measure filters
- **WHEN** a measure comparison and inequality are combined with a dimension filter for Presto
- **THEN** the SQL and ordered parameters equal their reviewed golden snapshot
#### Scenario: B0-PRESTO-011 Filter grouping
- **WHEN** top-level filters include an or group containing an and group for Presto
- **THEN** the SQL and ordered parameters equal their reviewed golden snapshot
#### Scenario: B0-PRESTO-012 Segment and result ordering
- **WHEN** a segment, two-member explicit order, limit and offset are requested for Presto
- **THEN** the SQL and ordered parameters equal their reviewed golden snapshot
#### Scenario: B0-PRESTO-013 Date operators
- **WHEN** inDateRange, notInDateRange, beforeDate, beforeOrOnDate, afterDate and afterOrOnDate are exercised for Presto
- **THEN** the SQL and ordered parameters equal their reviewed golden snapshot
#### Scenario: B0-PRESTO-014 Absolute range
- **WHEN** the October absolute range is compiled in UTC for Presto
- **THEN** the SQL and ordered parameters equal their reviewed golden snapshot
#### Scenario: B0-PRESTO-015 UTC granularities
- **WHEN** day, week, month, quarter and year are compiled in UTC for Presto
- **THEN** the SQL and ordered parameters equal their reviewed golden snapshot
#### Scenario: B0-PRESTO-016 Timezone precedence
- **WHEN** a month range is compiled with a context timezone and again with a different query timezone for Presto
- **THEN** the SQL and ordered parameters equal their reviewed golden snapshot

### Requirement: Presto offset session property
Presto SQL for a query with an offset SHALL use the OFFSET clause, as the pinned planner emits it. PrestoDB accepts that clause only when the session property offset_clause_enabled is true, so a Presto compile result for a query with an offset SHALL declare that property, the Presto execution manifest SHALL carry it from the compile results, and the executor of the SQL SHALL set it. The kernel SHALL NOT rewrite pagination to avoid the clause.
#### Scenario: B0-PRESTO-017 Declared offset property
- **WHEN** the Presto execution manifest is compiled
- **THEN** it declares the session property offset_clause_enabled as true, taken from the compile results of exactly its paginated cases, which are the cases that use the OFFSET clause

### Requirement: Presto B0 execution acceptance
Presto output SHALL be accepted by execution on Presto in tier T2, against expected results stated independently of the compiler.
#### Scenario: B0-PRESTO-003 Engine execution (T2)
- **WHEN** each golden execution case runs on a real Presto engine loaded with its fixture and the declared session properties
- **THEN** it executes and its column names and ordered rows equal the independently chosen expected result

### Requirement: Presto Cube Core comparison
Presto output SHALL be accepted against Cube Core at the release of the upstream pin, in tier T1: for every execution case Cube Core's SQL and parameters SHALL equal the kernel's, and that SQL SHALL return the case's expected result on Presto. The one permitted difference is the default row limit Cube's API adds to a query that sets none.
#### Scenario: B0-PRESTO-002 Cube Core comparison (T1)
- **WHEN** each execution case is put to pinned Cube Core and its SQL runs on a real Presto engine loaded with the case's fixture
- **THEN** its SQL and parameters equal the kernel's but for the default row limit, and its column names and ordered rows equal the expected result

### Requirement: Presto windows and shifts
Presto SHALL compile rolling windows and time shifts to the SQL Cube Core writes, with a time series the engine generates from the bounds of the range. It SHALL return unsupported, naming the measure, for what that SQL cannot express on Presto: an interval in weeks or quarters, a rolling window by week or by quarter, and a rolling window whose date range does not start a period of its granularity.
#### Scenario: B1-PRESTO-001 Presto window limits
- **WHEN** Presto is asked for a rolling window by week or by quarter, a window of two weeks, a shift of a quarter, and a rolling window by month, year or day over a range that starts inside a period
- **THEN** each returns unsupported naming the measure and the limit, while ranges that start a period, a range without a granularity, a time dimension without a range and a shift over any range compile
