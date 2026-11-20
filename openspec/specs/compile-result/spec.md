# compile-result Specification

## Purpose
TBD - created by archiving change add-complete-compile-result. Update Purpose after archive.

## Requirements

### Requirement: Complete compile result
Successful compilation SHALL return one SELECT, ordered parameter values, output columns with alias/member/type, used cubes and physical tables, and a fingerprint. The kernel SHALL perform no SQL execution or credential access.
#### Scenario: B0-RESULT-001 Parameter and column correspondence
- **WHEN** a query groups sales by region and filters using two distinct values
- **THEN** each placeholder maps to its ordered value, each selected column has matching metadata, and used cubes/tables match the compiled query
