# execution-cases Specification

## Purpose
Provide execution tiers with live-compiled cases and their expected results, so engine acceptance never depends on parsing stored snapshot text.

## Requirements

### Requirement: Live-compiled execution manifest
For a supported dialect the conformance crate SHALL emit a manifest compiled when it is requested. Each case SHALL carry a unique name, the scenario id it supports, its fixture statements, the compiled SQL, ordered parameters with an explicit type each, and expected column names and ordered rows chosen independently of the compiler. The same sources SHALL give a byte-identical manifest.
#### Scenario: B0-EXEC-001 Doris temporal manifest
- **WHEN** the Doris manifest is requested twice
- **THEN** both documents are byte-identical and hold every declared Doris case once, the thirteen temporal cases among them, each with its scenario id, fixture, typed ordered parameters and expected columns and rows

### Requirement: Manifest fails closed
The emitter SHALL fail without a manifest when a case does not compile, when a parameter has no declared manifest type, or when the dialect has no cases. It SHALL NOT emit a partial manifest.
#### Scenario: B0-EXEC-002 Unusable case
- **WHEN** a case fails to compile, and separately a manifest is requested for a dialect without cases
- **THEN** each request fails, names the case or dialect, and emits no manifest

### Requirement: Reviewed case goldens
The SQL and ordered parameters of every execution case SHALL equal a reviewed golden snapshot, so that a change in what an engine tier executes is visible in review. A case without a golden SHALL fail the check.
#### Scenario: B0-EXEC-003 Doris case goldens
- **WHEN** the Doris manifest is compiled
- **THEN** the SQL and ordered parameters of every case equal the reviewed golden snapshot, in manifest order

### Requirement: Cases shared across dialects
Every dialect's manifest SHALL be compiled from the same case declarations, so each engine answers the same queries with the same expected columns and rows; only fixtures, SQL and scenario ids differ. A manifest SHALL state the engine session properties its SQL needs, as an empty set when it needs none. The SQL and ordered parameters of every case of every dialect SHALL equal a reviewed golden snapshot.
#### Scenario: B0-EXEC-004 Presto manifest
- **WHEN** the Presto and Doris manifests are compiled
- **THEN** they hold the same case names in the same order with equal expected columns and rows, each declares its session properties, and the SQL and ordered parameters of every Presto case equal the reviewed golden snapshot

### Requirement: Sources for comparison
On request the emitter SHALL add to a manifest what an independent implementation needs to answer the same cases: the model text the cases were compiled against, its data sources, and each case's query in Cube's format with its effective default timezone and the security filters applied to it. Without the request the manifest SHALL be unchanged, and with it every other field SHALL be identical.
#### Scenario: B0-EXEC-005 Sources manifest
- **WHEN** the Presto manifest is requested with and without sources
- **THEN** the sources manifest carries a loadable model, its data sources, and a loadable query, a timezone and security filters for every case, and equals the plain manifest once those fields are removed

### Requirement: Domain scenario
The conformance crate SHALL hold a second scenario, a property-insurance model with its own fixtures and business queries, compiled for every supported dialect from the same declarations. Each query SHALL be recorded as compiled or refused with the class of its refusal, so that what the domain needs and the kernel does not compile is a reviewed list. The queries that compile SHALL be execution cases with expected results chosen independently of the compiler.
#### Scenario: B1-EXEC-001 Property-insurance scenario
- **WHEN** the scenario's queries are compiled for Presto and Doris
- **THEN** each is recorded as compiled or refused with its class, equal for both dialects unless a dialect limit is the reason, and the compiled ones are in both manifests with equal expected columns and rows

### Requirement: Manifest of a dialect that answers part of the cases
A dialect that declares features unsupported SHALL have a manifest of exactly the shared cases it compiles, in their shared order with the shared expected columns and rows, and a reviewed list that gives every other case its refusal. A case that fails any other way SHALL fail the manifest. A dialect MAY compile against the shared model rewritten for its engine's schema and time column.
#### Scenario: B0-EXEC-006 Druid manifest
- **WHEN** the Druid manifest is compiled twice and compared with the Presto manifest and the support list
- **THEN** both documents are identical, hold exactly the cases the list marks compiled with Presto's expected columns and rows, every other case has a refusal in the reviewed list, and each fixture statement replaces a Druid datasource with inline rows
