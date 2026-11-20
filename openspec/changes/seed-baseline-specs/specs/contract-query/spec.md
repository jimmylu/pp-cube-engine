## Purpose
Define B0 query inputs and preserve the pinned Cube query semantics.
## ADDED Requirements
### Requirement: Checks before compilation
Model validation SHALL precede query checks; invalid bundles return model issues. With a valid bundle, recognized Cube query features outside B0 SHALL return unsupported.
#### Scenario: B0-QUERY-009 Order of checks
- **WHEN** a query is compiled against an invalid bundle, and a query using a recognized Cube feature outside B0 is compiled against a valid one
- **THEN** the first returns model issues and the second returns unsupported, neither with SQL
