# Spec Delta

## ADDED Requirements

### Requirement: Views
A model MAY declare `views`. A view SHALL have a name and `cubes`, each with a `join_path` written `root.joined.joined`, `includes` as member names or `*`, and optionally `excludes` and `prefix`; it MAY have title, description, public and meta. A model without views SHALL serialize without the field. Expanded, a view SHALL be a cube whose members refer by join path to the dimensions and measures it takes, named `<cube>_<member>` under `prefix`.
#### Scenario: B1-MODEL-003 Views load and expand
- **WHEN** a model declares a view that takes named members of its root cube, all but one member of a joined cube under a prefix, and one member two joins away, in YAML and again as JSON
- **THEN** both formats give the same model, the expansion adds one cube whose dimensions and measures refer to the members taken by their join paths, with the declared granularities of a time dimension and no primary key, and a model without views serializes without the field
