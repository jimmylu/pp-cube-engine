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
