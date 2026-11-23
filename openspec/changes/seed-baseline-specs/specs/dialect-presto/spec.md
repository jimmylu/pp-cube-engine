## Purpose
Define the Presto output and acceptance boundary for baseline queries.
## ADDED Requirements
### Requirement: Presto B0 acceptance
Presto output SHALL be accepted by execution: against pinned Cube Core in tier T1 and, in tier T2, on Presto against expected results stated independently of the compiler.
#### Scenario: B0-PRESTO-002 Cube Core comparison (T1)
- **WHEN** each golden case and the SQL pinned Cube Core gives for the same model and query run on identical data
- **THEN** the results are equal value by value
#### Scenario: B0-PRESTO-003 Engine execution (T2)
- **WHEN** each golden execution case runs on a real Presto engine loaded with its fixture and the declared session properties
- **THEN** it executes and its column names and ordered rows equal the independently chosen expected result
