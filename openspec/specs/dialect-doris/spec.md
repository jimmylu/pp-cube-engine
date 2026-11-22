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
Doris SHALL return unsupported without SQL for offset without limit. Invalid models SHALL still produce model errors first. Supported temporal requests SHALL use the existing B0 validation and timezone precedence. This envelope SHALL NOT imply full B0 or engine execution acceptance.
#### Scenario: B0-DORIS-004 Initial unsupported envelope
- **WHEN** a Doris request has offset without limit
- **THEN** it returns unsupported without SQL

### Requirement: Doris dimension joins
Doris SHALL compile B0 joined dimensions through the existing same-source unique join graph, including joins needed only by security filters. Ordered parameters, selected columns and physical lineage SHALL be preserved. Ambiguous and cross-source paths, and a sole measure owned outside the join root, SHALL return query errors. Measures from multiple cubes SHALL return unsupported. Neither failure SHALL return SQL.
#### Scenario: B0-DORIS-005 Dimension join compilation
- **WHEN** a Doris request groups or filters by joined dimensions, or a security filter alone needs a two-hop dimension join
- **THEN** SQL equals its reviewed golden and parameters, columns and lineage preserve the joined dependencies
#### Scenario: B0-DORIS-006 Join boundary enforcement
- **WHEN** a Doris request uses an ambiguous path, cross-source path, a sole non-root measure or measures from multiple cubes
- **THEN** it fails with the declared error class without SQL

### Requirement: Doris literal pattern filters
Doris SHALL compile case-insensitive contains, startsWith, endsWith and their negations using bound parameters. Percent, underscore and backslash in input values SHALL be treated literally. Mandatory security patterns SHALL remain conjoined with the entire user filter expression, with ordered parameters and existing NULL semantics preserved.
#### Scenario: B0-DORIS-007 Literal pattern matching
- **WHEN** each pattern operator receives text containing percent, underscore and backslash
- **THEN** SQL equals its reviewed golden and the escaped values remain bound in placeholder order
#### Scenario: B0-DORIS-008 Security pattern conjunction
- **WHEN** a mandatory security pattern accompanies a nested user OR filter
- **THEN** SQL requires the security pattern AND the complete user OR group without changing parameter order

### Requirement: Doris temporal compilation
Doris SHALL compile time dimensions, day/week/month/quarter/year grouping and all B0 date operators using UTC millisecond bounds and the effective query timezone. Ordered bound parameters, metadata and security conjunction SHALL be preserved.
#### Scenario: B0-DORIS-009 Temporal SQL
- **WHEN** each B0 granularity and date operator compiles for Doris with a non-UTC timezone
- **THEN** SQL equals its reviewed golden and bounds remain ordered bound parameters
