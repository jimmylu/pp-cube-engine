## ADDED Requirements
### Requirement: Network service transport
The service SHALL expose Compile, ValidateModel and GetMeta through gRPC, with compilation dispatched to a fixed-size worker pool outside async runtime threads.
#### Scenario: B0-API-004 Network operations
- **WHEN** a gRPC client sends Compile, ValidateModel and GetMeta requests
- **THEN** the server maps them to the versioned service operations and dispatches compilation off the async runtime threads
