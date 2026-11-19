## Purpose
Give callers executable SQL and the metadata needed to interpret its output.
## ADDED Requirements
### Requirement: Complete compile result
Successful compilation SHALL return one SELECT, ordered parameter values, output columns with alias/member/type, used cubes and physical tables, and a fingerprint. The kernel SHALL perform no SQL execution or credential access.
#### Scenario: B0-RESULT-001 Parameter and column correspondence
- **WHEN** a query groups sales by region and filters using two distinct values
- **THEN** each placeholder maps to its ordered value, each selected column has matching metadata, and used cubes/tables match the compiled query
### Requirement: Classified errors
The facade SHALL distinguish model, query, unsupported and internal failures. Error messages SHALL expose member names and rules without physical table structure.
#### Scenario: B0-RESULT-002 Error boundaries
- **WHEN** an invalid model, an invalid query and an unsupported feature are each submitted, and the planner reports an error that is not a user error
- **THEN** they return model, query, unsupported and internal respectively, each without SQL or physical schema details
