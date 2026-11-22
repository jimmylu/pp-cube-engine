## Purpose
Define the Cube-compatible model inputs accepted by the B0 compile kernel.
## ADDED Requirements
### Requirement: Immutable model bundles
A model bundle SHALL be a set of models with a version and a caller-supplied digest. The kernel SHALL never mutate a bundle; a new version replaces it as a whole.
#### Scenario: B0-MODEL-003 Bundle replacement
- **WHEN** two bundle versions that define the same cube differently are compiled alternately
- **THEN** each result reflects its own bundle's definition and neither bundle changes
