## ADDED Requirements
### Requirement: Validation at the compile boundary
The public compile facade SHALL validate the complete typed model before checking queries or security filters and before invoking the planner. Invalid models SHALL return a model error containing the existing deterministic validation report. The public error message SHALL NOT include SQL or physical schema details. Successful output for valid models SHALL remain unchanged.
#### Scenario: B0-VALID-005 Invalid bundles cannot bypass validation
- **WHEN** an invalid typed bundle is compiled, including with an independently invalid query or security filter
- **THEN** compilation returns its model issues before query errors, without SQL, and leaves all inputs unchanged
