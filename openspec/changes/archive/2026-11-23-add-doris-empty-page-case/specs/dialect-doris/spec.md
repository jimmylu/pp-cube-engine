# Spec Delta

## MODIFIED Requirements

### Requirement: Doris B0 acceptance
Doris output SHALL be accepted by execution on Doris in tier T2, not inferred from MySQL syntax alone. Each case SHALL state its fixture and expected result independently of the compiler. Cube Core has no Doris dialect, so tier T1 does not apply.
#### Scenario: B0-DORIS-002 Engine execution (T2)
- **WHEN** each golden execution case runs on a real Doris engine loaded with its fixture
- **THEN** it executes and its column names and ordered rows equal the independently chosen expected result
#### Scenario: B0-DORIS-011 Page past the last group (T2)
- **WHEN** an ordered, grouped query with a limit and an offset equal to the number of groups runs on a real Doris engine
- **THEN** it executes and returns the expected columns and no rows
