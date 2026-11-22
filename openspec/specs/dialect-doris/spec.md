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
Doris SHALL return unsupported without SQL for multi-cube models, joins, time-typed model dimensions, timeDimensions, pattern/date filters or offset without limit. Invalid models SHALL still produce model errors first. This initial envelope SHALL NOT imply full B0 or engine execution acceptance.
#### Scenario: B0-DORIS-004 Initial unsupported envelope
- **WHEN** a Doris request exceeds the initial envelope, including through security filters
- **THEN** it returns unsupported without SQL
