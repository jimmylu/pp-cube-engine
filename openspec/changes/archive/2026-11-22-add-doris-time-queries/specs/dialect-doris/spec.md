## MODIFIED Requirements
### Requirement: Doris initial boundary
Doris SHALL return unsupported without SQL for offset without limit. Invalid models SHALL still produce model errors first. Supported temporal requests SHALL use the existing B0 validation and timezone precedence. This envelope SHALL NOT imply full B0 or engine execution acceptance.
#### Scenario: B0-DORIS-004 Initial unsupported envelope
- **WHEN** a Doris request has offset without limit
- **THEN** it returns unsupported without SQL
## ADDED Requirements
### Requirement: Doris temporal compilation
Doris SHALL compile time dimensions, day/week/month/quarter/year grouping and all B0 date operators using UTC millisecond bounds and the effective query timezone. Ordered bound parameters, metadata and security conjunction SHALL be preserved.
#### Scenario: B0-DORIS-009 Temporal SQL
- **WHEN** each B0 granularity and date operator compiles for Doris with a non-UTC timezone
- **THEN** SQL equals its reviewed golden and bounds remain ordered bound parameters
#### Scenario: B0-DORIS-010 Temporal execution smoke (T2)
- **WHEN** compiled temporal queries execute on a real local Doris fixture
- **THEN** calendar buckets, millisecond boundary inclusion and security restrictions equal independently chosen expected results
