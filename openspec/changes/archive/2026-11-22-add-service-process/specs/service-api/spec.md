## ADDED Requirements
### Requirement: Local service process
The binary SHALL require a model path, exact version, opaque digest and loopback listen address. It SHALL validate the model before readiness, expose the three existing gRPC operations and announce the bound address. Invalid startup SHALL exit unsuccessfully without readiness. Ctrl-C SHALL stop the server gracefully.
#### Scenario: B0-API-008 Runnable local service
- **WHEN** the binary starts with a valid model and explicit identity on a loopback socket
- **THEN** the announced address serves Compile, ValidateModel and GetMeta for that version, and Ctrl-C exits successfully
#### Scenario: B0-API-009 Invalid process startup
- **WHEN** arguments are malformed, a model is unavailable or invalid, or a socket cannot bind
- **THEN** the process exits unsuccessfully without announcing readiness
