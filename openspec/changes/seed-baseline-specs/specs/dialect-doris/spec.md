## Purpose
Define Doris acceptance independently of a nonexistent Cube Doris dialect.
## ADDED Requirements
### Requirement: Doris B0 compilation
The Doris dialect SHALL compile every member of the compile-b0 feature matrix to Doris SQL.
#### Scenario: B0-DORIS-001 Golden SQL
- **WHEN** a case for each matrix member, including each granularity with a non-UTC timezone, is compiled for Doris
- **THEN** the SQL equals its reviewed golden snapshot
### Requirement: Doris B0 acceptance
Doris output SHALL be accepted by execution against hand-written SQL on Doris in tier T2, not inferred from MySQL syntax alone. Cube Core has no Doris dialect, so tier T1 does not apply.
#### Scenario: B0-DORIS-002 Engine execution (T2)
- **WHEN** each golden case runs on Doris
- **THEN** it executes and its result equals the hand-written reference query on identical data

### Requirement: Doris temporal execution tier
The defined Doris execution tier SHALL run live-compiled temporal cases and fail on bucket, boundary or security regressions. Formal execution acceptance remains proposed until the test belongs to the execution layer and its invocation is integrated by a separate harness-only change.
#### Scenario: B0-DORIS-010 Temporal execution smoke (T2)
- **WHEN** compiled temporal queries execute on a real local Doris fixture
- **THEN** calendar buckets, millisecond boundary inclusion and security restrictions equal independently chosen expected results
