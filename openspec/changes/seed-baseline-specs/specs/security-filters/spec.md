## Purpose
Preserve caller-supplied row restrictions independently of user filter grouping.
## ADDED Requirements
### Requirement: Security filter placement
The facade SHALL append context security filters as top-level AND entries beside user filters. It SHALL NOT decide authorization or expose condition provenance.
#### Scenario: B0-SEC-001 User OR cannot bypass restrictions
- **WHEN** a user supplies (channel = C1 OR category = TOY) and context supplies region = R1
- **THEN** SQL requires region = R1 AND the entire user OR group, including when either user branch matches
### Requirement: Mandatory cube predicates
A mandatory predicate declared in cube sql SHALL remain effective for every query over that cube.
#### Scenario: B0-SEC-002 Cube-level restriction
- **WHEN** sales sql includes WHERE is_valid = 1 and the query omits filters
- **THEN** generated SQL retains that predicate and cannot include rows failing it
### Requirement: Security filters fail closed
The facade SHALL NOT drop or weaken a security filter. A security filter that is malformed, names an unknown member or cannot be compiled for the target dialect SHALL fail the compilation; one on a cube outside the query SHALL be applied through the declared join path or fail.
#### Scenario: B0-SEC-003 Unusable security filter
- **WHEN** context supplies, in turn, a malformed filter, a filter on an unknown member and a filter on a cube with no join path to the query
- **THEN** each compilation fails without SQL
