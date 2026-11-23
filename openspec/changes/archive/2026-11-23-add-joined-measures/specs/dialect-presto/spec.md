# Spec Delta

## ADDED Requirements

### Requirement: Presto Cube Core comparison
Presto output SHALL be accepted against Cube Core at the release of the upstream pin, in tier T1: for every execution case Cube Core's SQL and parameters SHALL equal the kernel's, and that SQL SHALL return the case's expected result on Presto. The one permitted difference is the default row limit Cube's API adds to a query that sets none.
#### Scenario: B0-PRESTO-002 Cube Core comparison (T1)
- **WHEN** each execution case is put to pinned Cube Core and its SQL runs on a real Presto engine loaded with the case's fixture
- **THEN** its SQL and parameters equal the kernel's but for the default row limit, and its column names and ordered rows equal the expected result
