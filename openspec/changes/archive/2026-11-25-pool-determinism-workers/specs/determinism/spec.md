# Spec Delta

## MODIFIED Requirements

### Requirement: Repeatable results
Identical model bundle, query, compile context and kernel version SHALL produce byte-identical compiled results across independent processes, without reading the clock or external state. The request id SHALL NOT affect the compiled result.
#### Scenario: B0-DET-001 Independent process comparison
- **WHEN** every golden case is compiled in the test process and, with identical inputs, in four other processes that are started for the test binary and kept for all its cases
- **THEN** SQL, ordered parameters, columns, used cubes, fingerprint and session properties are identical in all five, and a case that does not compile is not sent
