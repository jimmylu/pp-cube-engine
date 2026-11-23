# Spec Delta

## ADDED Requirements

### Requirement: Catalog listing and retirement
The catalog SHALL list its bundles ordered by version and SHALL retire a version on request. Handles obtained before a retirement SHALL keep their bundle unchanged. A retired version SHALL be unknown to later selection and MAY be registered again. Retiring an unknown version SHALL fail.
#### Scenario: B0-MODEL-009 List and retire
- **WHEN** three versions are registered out of order, one is retired while a handle to it is held, and it is registered again
- **THEN** listing is ordered by version, the held handle keeps its contents, the retired version cannot be selected or retired again, and the new registration is available
