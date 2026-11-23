# Spec Delta

## MODIFIED Requirements

### Requirement: Presto offset session property
Presto SQL for a query with an offset SHALL use the OFFSET clause, as the pinned planner emits it. PrestoDB accepts that clause only when the session property offset_clause_enabled is true, so a Presto compile result for a query with an offset SHALL declare that property, the Presto execution manifest SHALL carry it from the compile results, and the executor of the SQL SHALL set it. The kernel SHALL NOT rewrite pagination to avoid the clause.
#### Scenario: B0-PRESTO-017 Declared offset property
- **WHEN** the Presto execution manifest is compiled
- **THEN** it declares the session property offset_clause_enabled as true, taken from the compile results of exactly its paginated cases, which are the cases that use the OFFSET clause

## ADDED Requirements

### Requirement: Presto B0 execution acceptance
Presto output SHALL be accepted by execution on Presto in tier T2, against expected results stated independently of the compiler.
#### Scenario: B0-PRESTO-003 Engine execution (T2)
- **WHEN** each golden execution case runs on a real Presto engine loaded with its fixture and the declared session properties
- **THEN** it executes and its column names and ordered rows equal the independently chosen expected result
