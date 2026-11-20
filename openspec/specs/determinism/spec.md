# determinism Specification

## Purpose
Identify compilation inputs and build versions with a stable, correlation-independent fingerprint.

## Requirements

### Requirement: Fingerprint identity
The fingerprint SHALL cover the model bundle digest, query, dialect, timezone, security filters, kernel version and upstream pin, and SHALL exclude the request id.
#### Scenario: B0-DET-002 Fingerprint sensitivity
- **WHEN** each covered input is changed independently, and then only the request id is changed
- **THEN** the fingerprint changes for each covered input and stays the same for a new request id and for repeated identical inputs
