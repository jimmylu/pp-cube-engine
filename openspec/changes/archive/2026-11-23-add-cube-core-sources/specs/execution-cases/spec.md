# Spec Delta

## ADDED Requirements

### Requirement: Sources for comparison
On request the emitter SHALL add to a manifest what an independent implementation needs to answer the same cases: the model text the cases were compiled against, its data sources, and each case's query in Cube's format with its effective default timezone and the security filters applied to it. Without the request the manifest SHALL be unchanged, and with it every other field SHALL be identical.
#### Scenario: B0-EXEC-005 Sources manifest
- **WHEN** the Presto manifest is requested with and without sources
- **THEN** the sources manifest carries a loadable model, its data sources, and a loadable query, a timezone and security filters for every case, and equals the plain manifest once those fields are removed
