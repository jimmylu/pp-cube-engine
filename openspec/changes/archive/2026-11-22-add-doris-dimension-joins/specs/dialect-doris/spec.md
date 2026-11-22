## MODIFIED Requirements
### Requirement: Doris initial boundary
Doris SHALL return unsupported without SQL for time-typed model dimensions, timeDimensions, pattern/date filters or offset without limit. Invalid models SHALL still produce model errors first. This initial envelope SHALL NOT imply full B0 or engine execution acceptance.
#### Scenario: B0-DORIS-004 Initial unsupported envelope
- **WHEN** a Doris request exceeds the initial envelope, including through security filters
- **THEN** it returns unsupported without SQL

## ADDED Requirements
### Requirement: Doris dimension joins
Doris SHALL compile B0 joined dimensions through the existing same-source unique join graph, including joins needed only by security filters. Ordered parameters, selected columns and physical lineage SHALL be preserved. Ambiguous and cross-source paths SHALL return query errors; joined-cube measures SHALL remain unsupported without SQL.
#### Scenario: B0-DORIS-005 Dimension join compilation
- **WHEN** a Doris request groups or filters by joined dimensions, or a security filter alone needs a two-hop dimension join
- **THEN** SQL equals its reviewed golden and parameters, columns and lineage preserve the joined dependencies
#### Scenario: B0-DORIS-006 Join boundary enforcement
- **WHEN** a Doris request uses an ambiguous path, cross-source path or joined-cube measure
- **THEN** it fails with the declared error class without SQL
