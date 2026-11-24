# Spec Delta

## ADDED Requirements

### Requirement: Manifest of a dialect that answers part of the cases
A dialect that declares features unsupported SHALL have a manifest of exactly the shared cases it compiles, in their shared order with the shared expected columns and rows, and a reviewed list that gives every other case its refusal. A case that fails any other way SHALL fail the manifest. A dialect MAY compile against the shared model rewritten for its engine's schema and time column.
#### Scenario: B0-EXEC-006 Druid manifest
- **WHEN** the Druid manifest is compiled twice and compared with the Presto manifest and the support list
- **THEN** both documents are identical, hold exactly the cases the list marks compiled with Presto's expected columns and rows, every other case has a refusal in the reviewed list, and each fixture statement replaces a Druid datasource with inline rows
