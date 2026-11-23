## Purpose
Define the Presto output and acceptance boundary for baseline queries.
## ADDED Requirements
### Requirement: Presto Cube Core comparison
Presto output SHALL be accepted against pinned Cube Core in tier T1.
#### Scenario: B0-PRESTO-002 Cube Core comparison (T1)
- **WHEN** each golden case and the SQL pinned Cube Core gives for the same model and query run on identical data
- **THEN** the results are equal value by value
