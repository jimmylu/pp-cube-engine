# Spec Delta

## MODIFIED Requirements

### Requirement: Local service process
The binary SHALL require a loopback listen address, a dialect for every data source of its models, and one or more models each given as an exact version and a path, unless model management is enabled. It SHALL validate every model before readiness, expose the three existing gRPC operations and announce the bound address. Invalid startup SHALL exit unsuccessfully without readiness. Ctrl-C SHALL stop the server gracefully.
#### Scenario: B0-API-008 Runnable local service
- **WHEN** the binary starts with a valid model and explicit identity on a loopback socket
- **THEN** the announced address serves Compile, ValidateModel and GetMeta for that version, and Ctrl-C exits successfully
#### Scenario: B0-API-009 Invalid process startup
- **WHEN** arguments are malformed, a model is unavailable or invalid, a data source of the model has no dialect, or a socket cannot bind
- **THEN** the process exits unsuccessfully without announcing readiness

## ADDED Requirements

### Requirement: Model version lifecycle
The service SHALL publish, list and retire model versions while serving. Publishing SHALL validate first and SHALL NOT affect other versions. A version SHALL be immutable: publishing the same content again SHALL succeed without change and report that nothing was created; different content under an available version SHALL be refused. Retiring SHALL make the version unknown to new requests without failing compilations that already selected it. Listing SHALL be ordered by version.
#### Scenario: B0-API-013 Publish, list and retire
- **WHEN** versions are published, republished with the same and with different content, listed, compiled while others are published and retired, and retired
- **THEN** equal content is accepted without creating anything, different content is refused, listing is ordered, concurrent compilations of an available version all succeed, and a retired version is not found while the others still compile

### Requirement: Content digest
The service SHALL identify a version by a digest of its typed content, written as sha256: and 64 lowercase hex digits, equal for the same content in YAML or JSON and different for different content. ValidateModel SHALL return the digest of a valid model. A publish that states an expected digest SHALL publish nothing unless it equals the content digest.
#### Scenario: B0-API-014 Digest of content
- **WHEN** one model is written as commented YAML and as reordered JSON, a changed model is digested, and versions are published with a wrong and with the right expected digest
- **THEN** the two forms have one digest, the changed model another, the wrong expectation publishes nothing, and the right one publishes a version whose identity carries that digest

### Requirement: Opt-in management
ListModels, PublishModel and RetireModel SHALL be refused with a permission error unless the host enabled model management, which is off by default. Compile, ValidateModel and GetMeta SHALL be unaffected by the setting.
#### Scenario: B0-API-015 Management disabled
- **WHEN** a client calls the three management operations on a service that did not enable them
- **THEN** each is refused with a permission error, nothing is published, and validation and metadata requests are still answered
