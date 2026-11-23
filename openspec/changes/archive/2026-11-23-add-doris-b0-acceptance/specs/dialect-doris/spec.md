# Spec Delta

## ADDED Requirements

### Requirement: Doris B0 compilation
The Doris dialect SHALL compile every member of the compile-b0 feature matrix to Doris SQL, except offset without limit, which it declares unsupported.
#### Scenario: B0-DORIS-001 Golden SQL
- **WHEN** an execution case for each matrix member, including each granularity with a non-UTC timezone, is compiled for Doris
- **THEN** the SQL and ordered parameters of every case equal the reviewed golden snapshot

### Requirement: Doris B0 acceptance
Doris output SHALL be accepted by execution on Doris in tier T2, not inferred from MySQL syntax alone. Each case SHALL state its fixture and expected result independently of the compiler. Cube Core has no Doris dialect, so tier T1 does not apply.
#### Scenario: B0-DORIS-002 Engine execution (T2)
- **WHEN** each golden execution case runs on a real Doris engine loaded with its fixture
- **THEN** it executes and its column names and ordered rows equal the independently chosen expected result
