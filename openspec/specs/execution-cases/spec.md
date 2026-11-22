# execution-cases Specification

## Purpose
Provide execution tiers with live-compiled cases and their expected results, so engine acceptance never depends on parsing stored snapshot text.

## Requirements

### Requirement: Live-compiled execution manifest
For a supported dialect the conformance crate SHALL emit a manifest compiled when it is requested. Each case SHALL carry a unique name, the scenario id it supports, its fixture statements, the compiled SQL, ordered parameters with an explicit type each, and expected column names and ordered rows chosen independently of the compiler. The same sources SHALL give a byte-identical manifest.
#### Scenario: B0-EXEC-001 Doris temporal manifest
- **WHEN** the Doris manifest is requested twice
- **THEN** both documents are byte-identical and hold the thirteen temporal cases, each with its scenario id, fixture, typed ordered parameters and expected columns and rows

### Requirement: Manifest fails closed
The emitter SHALL fail without a manifest when a case does not compile, when a parameter has no declared manifest type, or when the dialect has no cases. It SHALL NOT emit a partial manifest.
#### Scenario: B0-EXEC-002 Unusable case
- **WHEN** a case fails to compile, and separately a manifest is requested for a dialect without cases
- **THEN** each request fails, names the case or dialect, and emits no manifest
