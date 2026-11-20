## Purpose
Give callers executable SQL and the metadata needed to interpret its output.
## ADDED Requirements
### Requirement: Classified errors
The facade SHALL distinguish model, query, unsupported and internal failures. Error messages SHALL expose member names and rules without physical table structure.
#### Scenario: B0-RESULT-002 Error boundaries
- **WHEN** an invalid model, an invalid query and an unsupported feature are each submitted, and the planner reports an error that is not a user error
- **THEN** they return model, query, unsupported and internal respectively, each without SQL or physical schema details
