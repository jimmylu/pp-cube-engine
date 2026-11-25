# Spec Delta

## ADDED Requirements

### Requirement: Stated relation to Cube Core
A case of a dialect's sources manifest MAY state that Cube Core refuses its query or writes different SQL. Only the Druid manifest SHALL state one, only for a reviewed list of cases, and the plain manifest SHALL be the same document without sources and relations.
#### Scenario: B0-EXEC-007 Relations in the Druid sources manifest
- **WHEN** the Druid and Presto sources manifests are emitted
- **THEN** the Druid cases for approximate counts and granularities counted from an origin state `refuses`, those with an offset state `differs`, every other Druid case and every Presto case states nothing, and removing sources and relations gives the plain manifest
