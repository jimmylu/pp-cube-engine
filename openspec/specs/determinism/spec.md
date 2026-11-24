# determinism Specification

## Purpose
Identify compilation inputs and build versions with a stable, correlation-independent fingerprint.

## Requirements

### Requirement: Fingerprint identity
The fingerprint SHALL cover the model bundle digest, query, dialect, timezone, security filters, kernel version and upstream pin, and SHALL exclude the request id.
#### Scenario: B0-DET-002 Fingerprint sensitivity
- **WHEN** each covered input is changed independently, and then only the request id is changed
- **THEN** the fingerprint changes for each covered input and stays the same for a new request id and for repeated identical inputs

### Requirement: Repeatable results
Identical model bundle, query, compile context and kernel version SHALL produce byte-identical compiled results across independent processes, without reading the clock or external state. The request id SHALL NOT affect the compiled result.
#### Scenario: B0-DET-001 Independent process comparison
- **WHEN** every golden case is compiled in the test process and, with identical inputs, in four other processes that are started for the test binary and kept for all its cases
- **THEN** SQL, ordered parameters, columns, used cubes, fingerprint and session properties are identical in all five, and a case that does not compile is not sent
