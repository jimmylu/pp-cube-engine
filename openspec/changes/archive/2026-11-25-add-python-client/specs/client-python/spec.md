# Spec Delta

## ADDED Requirements

### Requirement: Python client from the contract
The repository SHALL hold a Python client whose stubs are generated from the service's contract when it is built, with no copy of the contract, and which carries the kernel's version. It SHALL give a compiled query with its ordered parameters, session properties, columns and fingerprint, convert placeholders for drivers of other styles without touching quoted text, report an answer without SQL as a refusal of its kind, and refuse to send a token unencrypted.
#### Scenario: B1-CLIENT-003 The Python client and its back end follow the repository
- **WHEN** the Python client's build and the example's Python back end are read
- **THEN** the stubs are generated from the service's contract and not kept, the package and its module carry the kernel's version, and the back end reads the users, rules, questions, environment and warehouse the Java back end reads, hands parameters to the driver apart from the statement, and reports in the same words
