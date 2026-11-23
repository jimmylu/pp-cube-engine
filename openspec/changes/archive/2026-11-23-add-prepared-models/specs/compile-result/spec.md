# Spec Delta

## ADDED Requirements

### Requirement: Prepared model
The facade SHALL offer a prepared model that validates and digests a model once and compiles any number of queries against it. Preparing an invalid model SHALL return the model issues that compiling it would. Every compilation through a prepared model SHALL equal compiling the same query against the same model directly: SQL, parameters, columns, used cubes, session properties, fingerprint, and the class of any failure.
#### Scenario: B0-RESULT-004 Prepared compilation
- **WHEN** valid, invalid and unsupported queries are compiled for both dialects through a prepared model and directly, and an invalid model is prepared
- **THEN** each pair of results is equal including the fingerprint, and preparing the invalid model returns the issues direct compilation reports
