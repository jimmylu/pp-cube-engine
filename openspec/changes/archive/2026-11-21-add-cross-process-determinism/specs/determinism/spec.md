## ADDED Requirements
### Requirement: Repeatable results
Identical model bundle, query, compile context and kernel version SHALL produce byte-identical compiled results across independent processes, without reading the clock or external state. The request id SHALL NOT affect the compiled result.
#### Scenario: B0-DET-001 Independent process comparison
- **WHEN** every B0 golden case is compiled in four fresh processes with identical inputs
- **THEN** SQL, ordered parameters, columns, used cubes and fingerprint are identical
