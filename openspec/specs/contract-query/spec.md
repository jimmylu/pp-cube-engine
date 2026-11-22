# contract-query Specification

## Purpose
Define B0 query inputs and preserve the pinned Cube query semantics.

## Requirements

### Requirement: Value filters
The kernel SHALL accept filters in Cube's query format with the operators equals, notEquals, contains, notContains, startsWith, notStartsWith, endsWith, notEndsWith, gt, gte, lt, lte, set and notSet. A filter on a dimension SHALL restrict rows and a filter on a measure SHALL restrict groups. Filter values SHALL be bound as parameters, in the order of their placeholders in the SQL.
#### Scenario: B0-QUERY-006 Value operators
- **WHEN** queries exercise each of the fourteen operators on a member of a compatible type, on dimensions and on a measure
- **THEN** each compiles to the predicate pinned Cube semantics define for it, in WHERE for a dimension and HAVING for a measure, with its values as ordered parameters

### Requirement: Filter grouping
The kernel SHALL preserve nested and/or groups and combine top-level filters with AND.
#### Scenario: B0-QUERY-005 Filter grouping
- **WHEN** a query has two top-level filters, one of them an or group that contains an and group
- **THEN** SQL joins the top-level entries with AND and keeps each group's nesting and operator

### Requirement: Segments, order and row limits
The kernel SHALL accept segments, order, limit and offset in Cube's query format. Order SHALL name measures or dimensions of the query and SHALL be applied in the sequence it is written; an order on any other member SHALL be refused, not dropped. A query without an order SHALL get the planner's default order; an order written empty SHALL produce no ORDER BY.
#### Scenario: B0-QUERY-007 Segment, order, limit and offset
- **WHEN** a query names a segment, orders by two members in opposite directions, and sets a limit and an offset
- **THEN** SQL applies the segment's condition, orders by both members in the written sequence and direction, and carries the limit and the offset

### Requirement: Invalid filters
The kernel SHALL refuse a filter that names an unknown member, uses an operator outside the baseline, or applies an operator to a member whose type it does not fit. String operators fit string dimensions; comparison operators fit number dimensions and measures; equals and notEquals fit every member except a time dimension; set and notSet fit every member.
#### Scenario: B0-QUERY-002 Invalid query
- **WHEN** a query references an unknown member, uses an unrecognized operator, or applies an operator to an incompatible type
- **THEN** compilation returns a query error naming the member or rule

### Requirement: Date operators and granularities
The kernel SHALL accept timeDimensions in Cube's query format, each naming a time dimension with a granularity, a dateRange or both. Date operators SHALL be inDateRange, notInDateRange, beforeDate, beforeOrOnDate, afterDate and afterOrOnDate, and SHALL fit time dimensions only. B0 granularities SHALL be day, week, month, quarter and year. Recognized second, minute and hour granularities SHALL return unsupported; unknown names SHALL be query input errors.
#### Scenario: B0-QUERY-001 Supported query options
- **WHEN** queries exercise each date operator and each B0 granularity above on a time dimension
- **THEN** each compiles to the predicate or time truncation that pinned Cube semantics define for it
#### Scenario: B0-QUERY-008 Unrecognized granularity
- **WHEN** a query uses an unknown granularity such as fortnight
- **THEN** the query input is refused with an error naming the granularity rule

### Requirement: Checks before compilation
Model validation SHALL precede query checks; invalid bundles return model issues. With a valid bundle, recognized Cube query features outside B0 SHALL return unsupported.
#### Scenario: B0-QUERY-009 Order of checks
- **WHEN** a query is compiled against an invalid bundle, and a query using a recognized Cube feature outside B0 is compiled against a valid one
- **THEN** the first returns model issues and the second returns unsupported, neither with SQL

### Requirement: Recognized boundary inputs
Query loading SHALL preserve ungrouped booleans, timeDimensions.compareDateRange lists of absolute date-range pairs, and second, minute and hour granularities for compilation-time classification. ungrouped false SHALL preserve ordinary grouped compilation. Unknown fields, unknown granularity names and malformed field types SHALL remain input errors.
#### Scenario: B0-QUERY-011 Boundary input classification
- **WHEN** inputs exercise ungrouped true and false, compareDateRange with absolute pairs, all three sub-day granularities, an unknown field and granularity, a non-boolean ungrouped and a malformed comparison range
- **THEN** recognized well-typed inputs load; false compiles as grouped; other recognized boundary inputs return unsupported; unknown and malformed inputs fail loading

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
