# Spec Delta

## ADDED Requirements

### Requirement: Presto offset session property
Presto SQL for a query with an offset SHALL use the OFFSET clause, as the pinned planner emits it. PrestoDB accepts that clause only when the session property offset_clause_enabled is true, so the Presto execution manifest SHALL declare that property and the executor of the SQL SHALL set it. The kernel SHALL NOT rewrite pagination to avoid the clause.
#### Scenario: B0-PRESTO-017 Declared offset property
- **WHEN** the Presto execution manifest is compiled
- **THEN** it declares the session property offset_clause_enabled as true, and exactly its paginated cases use the OFFSET clause
