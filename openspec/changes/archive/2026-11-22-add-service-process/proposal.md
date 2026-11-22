## Why
The gRPC library can be hosted, but the shipped binary only prints its version. A local process makes existing B0 operations directly usable.
## What Changes
Load one YAML/JSON model file, register an explicit version and opaque digest, then serve the existing protocol on an explicit loopback socket. Reject invalid startup before readiness; handle Ctrl-C gracefully. No SQL output changes.
## Capabilities
### Modified Capabilities
- `service-api`: runnable local service lifecycle.
## Impact
Scope: crates/pp-cube-service/, Cargo.lock, this change and openspec/specs/service-api/spec.md. Product README lives inside the service crate. No harness/vendor edits. No public bind, authentication, TLS, reload, multiple startup versions, engine changes or SQL execution. Remains B0.
