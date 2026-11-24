## Purpose
Define how Druid output is accepted by execution.
## ADDED Requirements
### Requirement: Druid B0 acceptance
Druid output SHALL be accepted by execution: against pinned Cube Core in tier T1 and against expected results on Druid in tier T2.
#### Scenario: B0-DRUID-003 Cube Core comparison (T1)
- **WHEN** each Druid execution case is put to pinned Cube Core with its Druid dialect
- **THEN** its SQL and parameters equal the kernel's but for the default row limit
#### Scenario: B0-DRUID-004 Engine execution (T2)
- **WHEN** each Druid execution case runs on a real Druid engine loaded with its fixture and the declared query context
- **THEN** it executes and its column names and ordered rows equal the independently chosen expected result
