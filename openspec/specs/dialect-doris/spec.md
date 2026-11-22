# dialect-doris Specification

## Purpose
Provide a bounded initial Doris compilation path while keeping full B0 and execution acceptance separate.

## Requirements

### Requirement: Doris basic compilation
Doris SHALL compile single-cube models without joins or time dimensions using B0 aggregates, grouping, segments, scalar comparison/presence filters and ordered pagination. It SHALL preserve ordered security parameters, metadata and a dialect-specific fingerprint. Output SHALL use backtick identifiers and Doris LIMIT ordering.
#### Scenario: B0-DORIS-003 Basic SQL
- **WHEN** a basic single-cube aggregate query with grouping, scalar filters and security restrictions compiles for Doris
- **THEN** SQL equals its reviewed golden and parameters/metadata are preserved
### Requirement: Doris initial boundary
Doris SHALL return unsupported without SQL for time-typed model dimensions, timeDimensions, pattern/date filters or offset without limit. Invalid models SHALL still produce model errors first. This initial envelope SHALL NOT imply full B0 or engine execution acceptance.
#### Scenario: B0-DORIS-004 Initial unsupported envelope
- **WHEN** a Doris request exceeds the initial envelope, including through security filters
- **THEN** it returns unsupported without SQL

### Requirement: Doris dimension joins
Doris SHALL compile B0 joined dimensions through the existing same-source unique join graph, including joins needed only by security filters. Ordered parameters, selected columns and physical lineage SHALL be preserved. Ambiguous and cross-source paths SHALL return query errors; joined-cube measures SHALL remain unsupported without SQL.
#### Scenario: B0-DORIS-005 Dimension join compilation
- **WHEN** a Doris request groups or filters by joined dimensions, or a security filter alone needs a two-hop dimension join
- **THEN** SQL equals its reviewed golden and parameters, columns and lineage preserve the joined dependencies
#### Scenario: B0-DORIS-006 Join boundary enforcement
- **WHEN** a Doris request uses an ambiguous path, cross-source path or joined-cube measure
- **THEN** it fails with the declared error class without SQL
