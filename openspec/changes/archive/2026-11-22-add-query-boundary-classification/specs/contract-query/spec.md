## MODIFIED Requirements

### Requirement: Date operators and granularities
The kernel SHALL accept timeDimensions in Cube's query format, each naming a time dimension with a granularity, a dateRange or both. Date operators SHALL be inDateRange, notInDateRange, beforeDate, beforeOrOnDate, afterDate and afterOrOnDate, and SHALL fit time dimensions only. B0 granularities SHALL be day, week, month, quarter and year. Recognized second, minute and hour granularities SHALL return unsupported; unknown names SHALL be query input errors.
#### Scenario: B0-QUERY-001 Supported query options
- **WHEN** queries exercise each date operator and each B0 granularity above on a time dimension
- **THEN** each compiles to the predicate or time truncation that pinned Cube semantics define for it
#### Scenario: B0-QUERY-008 Unrecognized granularity
- **WHEN** a query uses an unknown granularity such as fortnight
- **THEN** the query input is refused with an error naming the granularity rule

## ADDED Requirements

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
