## Purpose
Define B0 query inputs and preserve the pinned Cube query semantics.
## ADDED Requirements
### Requirement: Checks before compilation
The kernel SHALL accept timeDimensions and timezone in Cube's query format. Model validation SHALL precede query checks; invalid bundles return model issues. With a valid bundle, recognized Cube query features outside B0 SHALL return unsupported.
#### Scenario: B0-QUERY-009 Order of checks
- **WHEN** a query is compiled against an invalid bundle, and a query using a recognized Cube feature outside B0 is compiled against a valid one
- **THEN** the first returns model issues and the second returns unsupported, neither with SQL
### Requirement: Date operators and granularities
Date operators SHALL be inDateRange, notInDateRange, beforeDate, beforeOrOnDate, afterDate and afterOrOnDate. Granularities SHALL be day, week, month, quarter and year.
#### Scenario: B0-QUERY-001 Supported query options
- **WHEN** queries exercise each date operator and each granularity above on a time dimension
- **THEN** each compiles to the predicate or time truncation that pinned Cube semantics define for it
#### Scenario: B0-QUERY-008 Unrecognized granularity
- **WHEN** a query uses a granularity outside the list
- **THEN** compilation returns a query error naming the rule
### Requirement: Absolute closed time ranges
The kernel SHALL reject relative date ranges with a query error and compile absolute date ranges as closed intervals, extending date-only end values to 23:59:59.999 in the effective timezone.
#### Scenario: B0-QUERY-003 October bounds
- **WHEN** dateRange is ["2026-10-01", "2026-10-31"] with UTC in both query and context
- **THEN** SQL uses >= and <= with bounds 2026-10-01T00:00:00.000 and 2026-10-31T23:59:59.999
#### Scenario: B0-QUERY-004 Relative range
- **WHEN** dateRange requests "last month"
- **THEN** compilation returns a query error rather than resolving it from the clock
