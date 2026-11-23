# Spec Delta

## ADDED Requirements

### Requirement: Declared session properties
A successful compilation SHALL list the engine session properties its SQL needs, as names and values, empty when it needs none. The list SHALL depend only on the dialect and the query. The kernel cannot apply them; the executor of the SQL SHALL.
#### Scenario: B0-RESULT-003 Session properties
- **WHEN** a query with and without an offset is compiled for Presto and for Doris
- **THEN** only the Presto result with an offset lists offset_clause_enabled as true, and every other result lists none
