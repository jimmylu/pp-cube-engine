# Spec Delta

## ADDED Requirements

### Requirement: Doris views
Doris SHALL compile queries on views with the same ordered parameters, columns and lineage as Presto for the same query.
#### Scenario: B1-DORIS-004 Views on Doris
- **WHEN** a view is queried by a joined dimension, with filters and a page, and for a sum over repeated rows, compiled for Doris
- **THEN** the SQL equals its reviewed golden, and parameters, columns and lineage equal those of the Presto compilation
