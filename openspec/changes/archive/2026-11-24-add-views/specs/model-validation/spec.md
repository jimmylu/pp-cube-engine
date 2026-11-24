# Spec Delta

## ADDED Requirements

### Requirement: View rules
A view's name SHALL differ from every cube's and every other view's. Each join path SHALL be a chain of joins the model declares, and all paths of a view SHALL start at one cube. `includes` and `excludes` SHALL name dimensions and measures of the cube the path ends at, and no two members of a view SHALL have one name. A model expression SHALL NOT write a join-path reference itself. Each violation SHALL be reported at its field.
#### Scenario: B1-VALID-004 View faults
- **WHEN** models independently give a view a missing or malformed includes, an unknown field, a non-boolean prefix, members of its own, a path that is not a declared chain or starts at a second cube, an unknown member or a segment in includes, an unknown member in excludes, two members of one name, or the name of a cube, and a measure writes a join-path reference
- **THEN** validation reports each at the offending field
