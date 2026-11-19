## Purpose
Expose the compile-only kernel through a versioned model service boundary.
## ADDED Requirements
### Requirement: Compile validation and metadata operations
The service SHALL expose Compile, ValidateModel and GetMeta. Compile SHALL select an available model version and return SQL or a classified failure without executing SQL; naming a version that is not available SHALL fail without SQL. ValidateModel SHALL return structured issues; GetMeta SHALL return public cubes, measures and dimensions only.
#### Scenario: B0-API-001 Service operations
- **WHEN** an available version is compiled, an unavailable version is named, an invalid bundle is validated, and metadata containing public and non-public members is requested
- **THEN** compilation returns the facade result, the unavailable version fails without SQL, validation returns located issues, and metadata excludes non-public cubes and members
### Requirement: Validate before availability
The service SHALL validate a bundle version before making it available to Compile and GetMeta, and SHALL NOT make an invalid version available. Versions already available SHALL be unaffected.
#### Scenario: B0-API-002 Invalid bundle version
- **WHEN** the service is given an invalid bundle version while a valid version is available
- **THEN** the validation issues are reported, the invalid version cannot be selected, and the valid version still compiles
### Requirement: Compile request propagation
The service SHALL pass the selected bundle, query, dialect, timezone, security filters and request id to the facade without dropping or weakening filters.
#### Scenario: B0-API-003 Security filters across the service
- **WHEN** Compile receives a user OR group with a context security filter, and then receives a malformed security filter
- **THEN** the first result requires the security filter AND the entire user OR group, and the second returns a classified failure without SQL
