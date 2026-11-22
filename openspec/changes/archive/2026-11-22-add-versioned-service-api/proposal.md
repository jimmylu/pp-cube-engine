## Why
Versioned bundles exist, but callers have no service boundary for compilation, validation or public metadata.
## What Changes
Add a synchronous Rust CubeService in pp-cube-service using the catalog and facade. Transfer all service-api behavior into this implementing change. This is a transport-independent slice; gRPC and worker dispatch remain proposed, not implemented.
## Capabilities
### New Capabilities
- `service-api`: typed version selection, validation, public metadata and compile context propagation.
## Impact
B0; crates/pp-cube-service, Cargo.lock and openspec only. No SQL, snapshots, vendor or harness changes. No network listener, serialization contract, persistence or authorization policy.
