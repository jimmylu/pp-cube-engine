## Purpose
Define B0 query inputs and preserve the pinned Cube query semantics.
## ADDED Requirements
### Requirement: Date operators and granularities
The kernel SHALL accept timeDimensions in Cube's query format, each naming a time dimension with a granularity, a dateRange or both. Date operators SHALL be inDateRange, notInDateRange, beforeDate, beforeOrOnDate, afterDate and afterOrOnDate, and SHALL fit time dimensions only. Granularities SHALL be day, week, month, quarter and year.
#### Scenario: B0-QUERY-001 Supported query options
- **WHEN** queries exercise each date operator and each granularity above on a time dimension
- **THEN** each compiles to the predicate or time truncation that pinned Cube semantics define for it
#### Scenario: B0-QUERY-008 Unrecognized granularity
- **WHEN** a query uses a granularity outside the list
- **THEN** compilation returns a query error naming the rule
### Requirement: Absolute closed time ranges
The kernel SHALL reject relative date ranges with a query error and compile absolute date ranges as closed intervals, extending date-only start values to 00:00:00.000 and date-only end values to 23:59:59.999 in the effective timezone. A date value SHALL be a date, or a date and a time, without a zone.
#### Scenario: B0-QUERY-003 October bounds
- **WHEN** dateRange is ["2026-10-01", "2026-10-31"] with UTC in both query and context
- **THEN** SQL uses >= and <= with bounds 2026-10-01T00:00:00.000 and 2026-10-31T23:59:59.999
#### Scenario: B0-QUERY-004 Relative range
- **WHEN** dateRange requests "last month"
- **THEN** compilation returns a query error rather than resolving it from the clock
### Requirement: Effective timezone
The kernel SHALL accept a timezone in the query and require one in the compile context. The query's timezone, when present, SHALL be the effective timezone; otherwise the context's. Time dimensions SHALL be truncated, and date values read, in the effective timezone. An unknown timezone name SHALL be a query error.
#### Scenario: B0-QUERY-010 Query and context timezone
- **WHEN** a month granularity with an October dateRange is compiled with a timezone ahead of UTC in the context only, and again with a different timezone in the query
- **THEN** the first SQL converts the time dimension and shifts the bounds by the context's timezone, and the second by the query's
