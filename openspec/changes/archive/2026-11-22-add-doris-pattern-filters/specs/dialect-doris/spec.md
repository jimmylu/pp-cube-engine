## MODIFIED Requirements
### Requirement: Doris initial boundary
Doris SHALL return unsupported without SQL for time-typed model dimensions, timeDimensions, date filters or offset without limit. Invalid models SHALL still produce model errors first. This initial envelope SHALL NOT imply full B0 or engine execution acceptance.
#### Scenario: B0-DORIS-004 Initial unsupported envelope
- **WHEN** a Doris request exceeds the initial envelope, including through security filters
- **THEN** it returns unsupported without SQL

## ADDED Requirements
### Requirement: Doris literal pattern filters
Doris SHALL compile case-insensitive contains, startsWith, endsWith and their negations using bound parameters. Percent, underscore and backslash in input values SHALL be treated literally. Mandatory security patterns SHALL remain conjoined with the entire user filter expression, with ordered parameters and existing NULL semantics preserved.
#### Scenario: B0-DORIS-007 Literal pattern matching
- **WHEN** each pattern operator receives text containing percent, underscore and backslash
- **THEN** SQL equals its reviewed golden and the escaped values remain bound in placeholder order
#### Scenario: B0-DORIS-008 Security pattern conjunction
- **WHEN** a mandatory security pattern accompanies a nested user OR filter
- **THEN** SQL requires the security pattern AND the complete user OR group without changing parameter order
