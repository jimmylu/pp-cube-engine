## Why
Versioned bundles exist, but callers have no service boundary for compilation, validation or public metadata.
## What Changes
Add a synchronous Rust CubeService in pp-cube-service using the catalog and facade. Transfer complete service-core requirements B0-API-001/002/003 from openspec/changes/seed-baseline-specs/specs/service-api/spec.md into this implementing change. Retain network transport and worker dispatch as the newly proposed B0-API-004 in that seed file. This is a transport-independent slice; gRPC and worker dispatch remain proposed, not implemented.
## Capabilities
### New Capabilities
- `service-api`: typed version selection, validation, public metadata and compile context propagation.
## Impact
B0; crates/pp-cube-service, Cargo.lock, openspec/changes/seed-baseline-specs/specs/service-api/spec.md, its transfer notes, this change folder and openspec/specs/service-api/spec.md only. No SQL, snapshots, vendor or harness changes. No network listener, serialization contract, persistence or authorization policy.
