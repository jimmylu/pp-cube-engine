# Spec Delta

## MODIFIED Requirements

### Requirement: Live-compiled execution manifest
For a supported dialect the conformance crate SHALL emit a manifest compiled when it is requested. Each case SHALL carry a unique name, the scenario id it supports, its fixture statements, the compiled SQL, ordered parameters with an explicit type each, and expected column names and ordered rows chosen independently of the compiler. The same sources SHALL give a byte-identical manifest.
#### Scenario: B0-EXEC-001 Doris temporal manifest
- **WHEN** the Doris manifest is requested twice
- **THEN** both documents are byte-identical and hold every declared Doris case once, the thirteen temporal cases among them, each with its scenario id, fixture, typed ordered parameters and expected columns and rows
