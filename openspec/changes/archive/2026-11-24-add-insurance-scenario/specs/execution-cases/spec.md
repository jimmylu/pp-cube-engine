# Spec Delta

## ADDED Requirements

### Requirement: Domain scenario
The conformance crate SHALL hold a second scenario, a property-insurance model with its own fixtures and business queries, compiled for every supported dialect from the same declarations. Each query SHALL be recorded as compiled or refused with the class of its refusal, so that what the domain needs and the kernel does not compile is a reviewed list. The queries that compile SHALL be execution cases with expected results chosen independently of the compiler.
#### Scenario: B1-EXEC-001 Property-insurance scenario
- **WHEN** the scenario's queries are compiled for Presto and Doris
- **THEN** each is recorded as compiled or refused with its class, equal for both dialects unless a dialect limit is the reason, and the compiled ones are in both manifests with equal expected columns and rows
