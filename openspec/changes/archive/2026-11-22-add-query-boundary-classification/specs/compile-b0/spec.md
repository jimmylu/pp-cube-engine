## ADDED Requirements

### Requirement: B0 boundary
The B0 feature matrix SHALL be: the six aggregate types, filtered measures, number formulas, joined dimensions, every contract-query filter operator, and/or groups, segments, absolute time ranges, the five granularities, timezone conversion, explicit order, limit and offset. The kernel SHALL return unsupported, without SQL, for B1 query features and for matrix members outside the target dialect's declared support.
#### Scenario: B0-AGG-003 Higher-level query
- **WHEN** a query with a valid B0 bundle requests either measures from more than one cube in an otherwise valid query connected by a unique usable join tree on one data source, or ungrouped output, compareDateRange or a recognized Cube granularity outside B0
- **THEN** compilation returns unsupported without SQL
- **AND** when multi-cube measures are the only requested feature outside B0, query validation takes precedence: invalid members or security filters, disconnected cubes, ambiguous join paths and differing data sources return a query error without SQL
