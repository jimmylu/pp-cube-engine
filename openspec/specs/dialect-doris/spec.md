# dialect-doris Specification

## Purpose
Define Doris compilation and its acceptance by execution, independently of a nonexistent Cube Doris dialect.

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
Doris SHALL compile B0 joined dimensions through the existing same-source unique join graph, including joins needed only by security filters. Ordered parameters, selected columns and physical lineage SHALL be preserved. Unconnected, ambiguous and cross-source paths SHALL return query errors, for each fact of a query over several. A filter on a count over repeated rows SHALL return unsupported. Neither failure SHALL return SQL.
#### Scenario: B0-DORIS-005 Dimension join compilation
- **WHEN** a Doris request groups or filters by joined dimensions, or a security filter alone needs a two-hop dimension join
- **THEN** SQL equals its reviewed golden and parameters, columns and lineage preserve the joined dependencies
#### Scenario: B0-DORIS-006 Join boundary enforcement
- **WHEN** a Doris request uses unconnected cubes, an ambiguous path, a cross-source path, two facts with a dimension of one of them only, or a filter on a count from a cube reached through a many_to_one join
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

### Requirement: Doris temporal execution tier
The Doris execution tier SHALL execute the live-compiled temporal cases of the execution manifest on a real Doris engine and fail on a bucket, boundary or security regression. This SHALL NOT imply full B0 execution acceptance.
#### Scenario: B0-DORIS-010 Temporal execution smoke (T2)
- **WHEN** compiled temporal queries execute on a real local Doris fixture
- **THEN** calendar buckets, millisecond boundary inclusion and security restrictions equal independently chosen expected results

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
#### Scenario: B0-DORIS-011 Page past the last group (T2)
- **WHEN** an ordered, grouped query with a limit and an offset equal to the number of groups runs on a real Doris engine
- **THEN** it executes and returns the expected columns and no rows

### Requirement: Doris queries over several facts
The Doris dialect SHALL compile every compile-b1 query over several facts to Doris SQL with its own identifier quoting and its limit and offset order, and with the same ordered parameters, columns and lineage as Presto for the same query.
#### Scenario: B1-DORIS-001 Multi-fact golden SQL
- **WHEN** two facts by a shared dimension, their totals, a filtered and paged combination, and a sum over repeated rows are compiled for Doris
- **THEN** the SQL equals its reviewed golden, and parameters, columns and lineage equal those of the Presto compilation
