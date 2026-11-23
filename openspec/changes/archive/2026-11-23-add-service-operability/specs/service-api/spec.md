# Spec Delta

## MODIFIED Requirements

### Requirement: Local service process
The binary SHALL require a listen address, a dialect for every data source of its models, and one or more models each given as an exact version and a path, unless model management is enabled. It SHALL validate every model before readiness, expose the service operations and the health protocol, and announce the bound address. Invalid startup SHALL exit unsuccessfully without readiness. An interrupt or terminate signal SHALL stop the server gracefully.
#### Scenario: B0-API-008 Runnable local service
- **WHEN** the binary starts with a valid model and explicit identity on a loopback socket
- **THEN** the announced address serves Compile, ValidateModel and GetMeta for that version, and Ctrl-C exits successfully
#### Scenario: B0-API-009 Invalid process startup
- **WHEN** arguments are malformed, a model is unavailable or invalid, a data source of the model has no dialect, or a socket cannot bind
- **THEN** the process exits unsuccessfully without announcing readiness

## ADDED Requirements

### Requirement: Access tokens
The gRPC service SHALL support a read token and an admin token presented as bearer credentials. With a read token configured, Compile, ValidateModel and GetMeta SHALL require the read or the admin token. With any token configured, management operations SHALL require the admin token. A missing or unknown credential SHALL be an authentication error, a read token on a management operation a permission error. With no token configured, callers are trusted. The health protocol SHALL need no token.
#### Scenario: B0-API-016 Token roles
- **WHEN** clients call with no credential, another scheme, an unknown token, the read token and the admin token
- **THEN** the first three are unauthenticated, the read token compiles and reads metadata but is denied management, the admin token may do both, and a health probe is answered without a token

### Requirement: Health protocol
The service SHALL answer the standard gRPC health Check for the whole server and for its own service name with serving, and with not serving once the host has begun to drain. An unknown service name SHALL be not found. Watch SHALL be unimplemented.
#### Scenario: B0-API-017 Health check
- **WHEN** a probe checks the server, the service name and an unknown name, calls Watch, and checks again after draining has begun
- **THEN** it receives serving twice, not found, unimplemented, and then not serving

### Requirement: Request telemetry
The service SHALL count every finished request by operation and outcome, record its duration, and emit one structured log line with operation, outcome, duration and, when present, request id, model version, known dialect and result fingerprint. Telemetry SHALL NOT contain SQL, parameter or filter values, table names, model content or credentials. Caller-chosen text SHALL be length-limited and unable to split or forge a line.
#### Scenario: B0-API-018 Counted and logged
- **WHEN** a restricted query compiles, an invalid query fails, an unknown version is named and an unauthenticated call is refused
- **THEN** each produces one log line and one count under its outcome, the lines carry the listed fields, and neither lines nor metrics contain the filter value, table name, SQL, members or tokens

### Requirement: Bounded requests
The service SHALL bound how long a caller waits for a compilation, the size of a request, and the number of available versions. A compilation that overruns SHALL return a deadline error and the service SHALL keep working. An oversized request SHALL be refused before it is interpreted. Publishing beyond the version limit SHALL be refused until a version is retired; republishing available content SHALL not count.
#### Scenario: B0-API-019 Limits
- **WHEN** a compilation cannot start within the deadline, a request exceeds the size limit, and a version is published beyond the limit and again after one is retired
- **THEN** the first is a deadline error after which requests succeed, the second is refused, the third is refused and then accepted

### Requirement: Deployable host
The binary SHALL listen beyond loopback only with a TLS certificate and key and a read token, SHALL read tokens from files, and SHALL refuse management with tokens but no admin token. It SHALL optionally serve metrics in the Prometheus text format on a separate address, log request events as JSON lines on standard error, and on a terminate or interrupt signal report not serving, finish running requests within a drain time, and exit.
#### Scenario: B0-API-020 Encrypted host
- **WHEN** the binary is started beyond loopback without TLS or without a token, and then with both, a metrics address and management
- **THEN** the first starts fail without readiness, and the last serves encrypted identified callers, refuses plaintext and unidentified ones, answers a probe, serves metrics, logs JSON events, and exits successfully on a terminate signal
