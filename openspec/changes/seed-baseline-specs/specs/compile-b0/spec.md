## Purpose
Specify the first supported query level without promising B1 behavior.
## ADDED Requirements
### Requirement: B0 boundary
The B0 feature matrix SHALL be: the six aggregate types, filtered measures, number formulas, joined dimensions, every contract-query filter operator, and/or groups, segments, absolute time ranges, the five granularities, timezone conversion, explicit order, limit and offset. The kernel SHALL return unsupported, without SQL, for B1 query features and for matrix members outside the target dialect's declared support.
#### Scenario: B0-AGG-003 Higher-level query
- **WHEN** a query with a valid B0 bundle requests measures from more than one cube, ungrouped output, compareDateRange or a recognized Cube granularity outside B0
- **THEN** compilation returns unsupported without SQL
