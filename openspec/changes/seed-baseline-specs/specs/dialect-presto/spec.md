## Purpose
Define the Presto output and acceptance boundary for baseline queries.
## ADDED Requirements
### Requirement: Presto B0 compilation
The Presto dialect SHALL compile every member of the compile-b0 feature matrix to Presto SQL.
#### Scenario: B0-PRESTO-001 Golden SQL
- **WHEN** a case for each matrix member, including each granularity with a non-UTC timezone, is compiled for Presto
- **THEN** the SQL equals its reviewed golden snapshot
### Requirement: Presto B0 acceptance
Presto output SHALL be accepted by execution: against pinned Cube Core in tier T1 and against hand-written SQL on Presto in tier T2.
#### Scenario: B0-PRESTO-002 Cube Core comparison (T1)
- **WHEN** each golden case and the SQL pinned Cube Core gives for the same model and query run on identical data
- **THEN** the results are equal value by value
#### Scenario: B0-PRESTO-003 Engine execution (T2)
- **WHEN** each golden case runs on Presto
- **THEN** it executes and its result equals the hand-written reference query on identical data
