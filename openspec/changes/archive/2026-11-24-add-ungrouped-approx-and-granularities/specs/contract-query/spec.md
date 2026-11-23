# Spec Delta

## MODIFIED Requirements

### Requirement: Date operators and granularities
The kernel SHALL accept timeDimensions in Cube's query format, each naming a time dimension with a granularity, a dateRange or both. Date operators SHALL be inDateRange, notInDateRange, beforeDate, beforeOrOnDate, afterDate and afterOrOnDate, on time dimensions only. Predefined granularities SHALL be day, week, month, quarter and year; second, minute and hour SHALL return unsupported. Another name SHALL be one the time dimension declares; text that is not a name SHALL be an input error.
#### Scenario: B0-QUERY-001 Supported query options
- **WHEN** queries exercise each date operator and each B0 granularity above on a time dimension
- **THEN** each compiles to the predicate or time truncation that pinned Cube semantics define for it
#### Scenario: B0-QUERY-008 Unrecognized granularity
- **WHEN** a query writes a granularity that cannot be a name, and another names a granularity such as fortnight that its time dimension does not declare
- **THEN** the first is refused where the query is read with an error naming the granularity rule, and the second is a query error naming the granularity and the dimension

### Requirement: Recognized boundary inputs
Query loading SHALL preserve ungrouped booleans, timeDimensions.compareDateRange lists of absolute date-range pairs, and second, minute and hour granularities for compilation-time classification. ungrouped false SHALL preserve ordinary grouped compilation. Unknown fields, text that cannot name a granularity and malformed field types SHALL remain input errors.
#### Scenario: B0-QUERY-011 Boundary input classification
- **WHEN** inputs exercise ungrouped false, compareDateRange with absolute pairs, all three sub-day granularities, an unknown field, a granularity that is not a name, a non-boolean ungrouped and a malformed comparison range
- **THEN** recognized well-typed inputs load; false compiles as grouped; compareDateRange and sub-day granularities return unsupported; unknown and malformed inputs fail loading

## ADDED Requirements

### Requirement: Declared granularities and ungrouped in queries
A query SHALL keep a granularity that is not predefined by its name, written back as written, and `ungrouped: true` as a query to compile. Neither SHALL be refused before compilation.
#### Scenario: B1-QUERY-001 Granularity by name
- **WHEN** an ungrouped query names a declared granularity and a predefined one, and is serialized and read again
- **THEN** both granularities and the ungrouped flag are preserved, no feature is reported unsupported, and text that cannot be a name fails loading
