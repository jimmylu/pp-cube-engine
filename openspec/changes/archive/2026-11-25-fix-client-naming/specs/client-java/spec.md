# Spec Delta

## MODIFIED Requirements

### Requirement: Java client from the contract
The repository SHALL hold a Java client whose stubs are generated from the service's contract in the same checkout and which carries the kernel's version. Its packages SHALL be named after the contract's package and each source file SHALL sit where its package says. It SHALL give a compiled query with its ordered typed parameters, session properties, columns and fingerprint, bind parameters by type, report an answer without SQL as a refusal of its kind, and refuse to send a token unencrypted.
#### Scenario: B1-CLIENT-002 The client and the example follow the repository
- **WHEN** the client's build, the example's build, their Java sources and the example's service arguments are read
- **THEN** the client is generated from the service's contract, both carry the kernel's version, every Java file's package begins with `dev.ppcube.` and equals its directory, the contract names `dev.ppcube.engine.v1` as its Java package, and the example's service takes the arguments of the shipped deployment but for how often it reads its store
