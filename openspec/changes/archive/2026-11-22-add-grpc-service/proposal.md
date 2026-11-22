## Why
The versioned core and fixed pool are implemented; network callers cannot use them.
## What Changes
Implement gRPC Compile, ValidateModel and GetMeta with generated client/server bindings. Transfer complete B0-API-004 from openspec/changes/seed-baseline-specs/specs/service-api/spec.md; delete that empty delta and update seed notes.
## Capabilities
### Modified Capabilities
- `service-api`: actual gRPC transport over the existing service and fixed workers.
## Impact
B0; crates/pp-cube-service (proto, build script, runtime dependencies, transport and tests), Cargo.lock, this change, main service-api spec and the named seed spec/notes only. No harness/vendor/snapshot edits. No standalone executable configuration, authentication, TLS, model upload or SQL execution. Host integration owns a preloaded service and listener.
