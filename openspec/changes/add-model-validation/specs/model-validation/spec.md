## Purpose
Report structural model faults together, with stable locations and rules, before callers request compilation.

## ADDED Requirements
### Requirement: Structured complete validation
Validation SHALL return all detected issues with object path, rule id and message. It SHALL check required fields, types, reference resolution, join targets, primary keys at both join ends, one join per cube pair and the contract-model field allowlist.
#### Scenario: B0-VALID-001 Invalid definitions
- **WHEN** models independently contain missing fields, invalid types, unresolved references, absent join targets, missing join keys, duplicate joins or unknown fields
- **THEN** each is rejected with a located issue, and a model containing several such faults returns all detected issues
