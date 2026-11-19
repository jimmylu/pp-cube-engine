## Purpose
Make compilation repeatable and identify the inputs and versions behind it.
## ADDED Requirements
### Requirement: Repeatable results
Identical model bundle, query, compile context and kernel version SHALL produce byte-identical compiled results across independent processes, without reading the clock or external state. The request id SHALL NOT affect the compiled result.
#### Scenario: B0-DET-001 Independent process comparison
- **WHEN** every B0 golden case is compiled in four fresh processes with identical inputs
- **THEN** SQL, ordered parameters, columns, used cubes and fingerprint are identical
### Requirement: Fingerprint identity
The fingerprint SHALL cover the model bundle digest, query, dialect, timezone, security filters, kernel version and upstream pin, and SHALL exclude the request id.
#### Scenario: B0-DET-002 Fingerprint sensitivity
- **WHEN** each covered input is changed independently, and then only the request id is changed
- **THEN** the fingerprint changes for each covered input and stays the same for a new request id and for repeated identical inputs
