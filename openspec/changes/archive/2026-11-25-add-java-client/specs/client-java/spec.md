# Spec Delta

## ADDED Requirements

### Requirement: Java client from the contract
The repository SHALL hold a Java client whose stubs are generated from the service's contract in the same checkout and which carries the kernel's version. It SHALL give a compiled query with its ordered typed parameters, session properties, columns and fingerprint, bind parameters by type, report an answer without SQL as a refusal of its kind, and refuse to send a token unencrypted.
#### Scenario: B1-CLIENT-002 The client and the example follow the repository
- **WHEN** the client's build, the example's build and the example's service arguments are read
- **THEN** the client is generated from the service's contract, both carry the kernel's version, and the example's service takes the arguments of the shipped deployment but for how often it reads its store

### Requirement: Runnable example
The repository SHALL hold an example that runs: a model store in the published layout, the service as deployed, a warehouse, and a Java back end that compiles each user's questions under their row-level rules, executes the SQL with bound parameters and session properties, and checks the rows. Its model SHALL be the property-insurance scenario's and its expected answers those of the executed cases.
#### Scenario: B1-CLIENT-001 The example asks the scenario
- **WHEN** the example's store is assembled and each of its questions is compiled for its user's rules
- **THEN** the model equals the scenario's, fifteen questions compile with the stated columns, two are refused with the stated kind, and where a question is an executed case its query and expected rows are that case's, under the security case's rule for the restricted user
