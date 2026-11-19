## Purpose
Define B0 query inputs and preserve the pinned Cube query semantics.
## ADDED Requirements
### Requirement: B0 query vocabulary
The kernel SHALL accept measures, dimensions, segments, filters, timeDimensions, order, limit, offset and timezone in Cube's query format. It SHALL preserve nested and/or groups and combine top-level filters with AND. Model validation SHALL precede query checks; invalid bundles return model issues. With a valid bundle, recognized Cube query features outside B0 SHALL return unsupported.
#### Scenario: B0-QUERY-005 Filter grouping
- **WHEN** a query has two top-level filters, one of them an or group that contains an and group
- **THEN** SQL joins the top-level entries with AND and keeps each group's nesting and operator
### Requirement: Filter operators and granularities
Filter operators SHALL be equals, notEquals, contains, notContains, startsWith, notStartsWith, endsWith, notEndsWith, gt, gte, lt, lte, set, notSet, inDateRange, notInDateRange, beforeDate, beforeOrOnDate, afterDate and afterOrOnDate. Granularities SHALL be day, week, month, quarter and year.
#### Scenario: B0-QUERY-001 Supported query options
- **WHEN** queries exercise each operator and granularity above, a segment, explicit order, limit and offset on compatible members
- **THEN** each compiles to the predicate, time truncation, ORDER BY or row-limit clause that pinned Cube semantics define for it
#### Scenario: B0-QUERY-002 Invalid query
- **WHEN** a query references an unknown member, uses an unrecognized operator or granularity, or applies an operator to an incompatible type
- **THEN** compilation returns a query error naming the member or rule
### Requirement: Absolute closed time ranges
The kernel SHALL reject relative date ranges with a query error and compile absolute date ranges as closed intervals, extending date-only end values to 23:59:59.999 in the effective timezone.
#### Scenario: B0-QUERY-003 October bounds
- **WHEN** dateRange is ["2026-10-01", "2026-10-31"] with UTC in both query and context
- **THEN** SQL uses >= and <= with bounds 2026-10-01T00:00:00.000 and 2026-10-31T23:59:59.999
#### Scenario: B0-QUERY-004 Relative range
- **WHEN** dateRange requests "last month"
- **THEN** compilation returns a query error rather than resolving it from the clock
