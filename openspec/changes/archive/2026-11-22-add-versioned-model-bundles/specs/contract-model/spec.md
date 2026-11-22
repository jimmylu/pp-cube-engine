## ADDED Requirements

### Requirement: Immutable model bundles
A model bundle SHALL own a validated set of models, an exact nonblank version and a nonblank caller-supplied digest. The kernel SHALL expose only shared read access to bundle contents. Digests SHALL be retained as opaque caller metadata without claiming content verification; compilation fingerprints SHALL continue to reflect semantic model content. A new version replaces the complete model set selected by the caller, never part of an existing bundle.
#### Scenario: B0-MODEL-003 Bundle replacement
- **WHEN** two bundle versions that define the same cube differently are compiled alternately
- **THEN** each result reflects its own bundle's definition and neither bundle changes

### Requirement: Validated catalog registration
The catalog SHALL accept only validated bundles, retain multiple versions and reject duplicate version registration without replacing existing contents, even when digests agree. Blank version or digest identifiers SHALL fail; nonblank identifiers SHALL be retained without normalization. Invalid model bundles SHALL report the existing structured validation issues and SHALL NOT become available or affect available versions.
#### Scenario: B0-MODEL-007 Invalid registration isolation
- **WHEN** a valid version is registered, an invalid model and blank identifiers are offered, and the valid version is selected again
- **THEN** invalid input is rejected with classified errors, model issues retain locations, and the valid version remains unchanged and selectable

### Requirement: Explicit version selection
Catalog lookup SHALL return a shared immutable bundle for the exact requested version or an unknown-version error without selecting another version. Duplicate registration SHALL return a duplicate-version error. Existing bundle handles SHALL remain usable after additional registrations and after the catalog is dropped.
#### Scenario: B0-MODEL-008 Exact selection and handle lifetime
- **WHEN** a duplicate version is offered with different contents, an unknown version is selected, another version is registered, and the catalog is dropped
- **THEN** duplicate and unknown selection fail, the original version is not replaced, and retained handles preserve their versions, digests and model contents
