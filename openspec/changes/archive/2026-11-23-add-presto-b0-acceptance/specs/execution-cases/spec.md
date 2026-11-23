# Spec Delta

## ADDED Requirements

### Requirement: Cases shared across dialects
Every dialect's manifest SHALL be compiled from the same case declarations, so each engine answers the same queries with the same expected columns and rows; only fixtures, SQL and scenario ids differ. A manifest SHALL state the engine session properties its SQL needs, as an empty set when it needs none. The SQL and ordered parameters of every case of every dialect SHALL equal a reviewed golden snapshot.
#### Scenario: B0-EXEC-004 Presto manifest
- **WHEN** the Presto and Doris manifests are compiled
- **THEN** they hold the same case names in the same order with equal expected columns and rows, each declares its session properties, and the SQL and ordered parameters of every Presto case equal the reviewed golden snapshot
