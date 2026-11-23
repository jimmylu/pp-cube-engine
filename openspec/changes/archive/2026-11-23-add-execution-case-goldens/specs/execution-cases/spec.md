# Spec Delta

## ADDED Requirements

### Requirement: Reviewed case goldens
The SQL and ordered parameters of every execution case SHALL equal a reviewed golden snapshot, so that a change in what an engine tier executes is visible in review. A case without a golden SHALL fail the check.
#### Scenario: B0-EXEC-003 Doris case goldens
- **WHEN** the Doris manifest is compiled
- **THEN** the SQL and ordered parameters of every case equal the reviewed golden snapshot, in manifest order
