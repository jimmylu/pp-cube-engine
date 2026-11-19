## Purpose
Define the Druid single-cube acceptance boundary without promising B1 SQL.
## ADDED Requirements
### Requirement: Druid single-cube B0
The Druid dialect SHALL compile every member of the compile-b0 feature matrix except joined dimensions to Druid SQL.
#### Scenario: B0-DRUID-001 Golden SQL
- **WHEN** a case for each single-cube matrix member, including each granularity with a non-UTC timezone, is compiled for Druid
- **THEN** the SQL equals its reviewed golden snapshot
### Requirement: Druid support declaration
The Druid dialect SHALL declare joined dimensions unsupported in B0. A Druid query that needs a join, including one a security filter introduces, SHALL return unsupported without SQL.
#### Scenario: B0-DRUID-002 Joined query
- **WHEN** a Druid query groups or filters sales by a store dimension reached through a join
- **THEN** compilation returns unsupported without SQL
### Requirement: Druid B0 acceptance
Druid output SHALL be accepted by execution: against pinned Cube Core in tier T1 and against hand-written SQL on Druid in tier T2.
#### Scenario: B0-DRUID-003 Cube Core comparison (T1)
- **WHEN** each golden case and the SQL pinned Cube Core gives for the same model and query run on identical data
- **THEN** the results are equal value by value
#### Scenario: B0-DRUID-004 Engine execution (T2)
- **WHEN** each golden case runs on Druid
- **THEN** it executes and its result equals the hand-written reference query on identical data
