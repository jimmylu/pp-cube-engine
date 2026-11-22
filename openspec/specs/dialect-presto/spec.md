# dialect-presto Specification

## Purpose
Define the Presto output and acceptance boundary for baseline queries.
## Requirements
### Requirement: Presto B0 compilation
The Presto dialect SHALL compile every member of the compile-b0 feature matrix to Presto SQL.
#### Scenario: B0-PRESTO-001 Golden SQL
- **WHEN** a case for each matrix member, including each granularity with a non-UTC timezone, is compiled for Presto
- **THEN** the SQL equals its reviewed golden snapshot
