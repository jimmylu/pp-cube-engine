## Why
The versioned service compiles synchronously. Async hosts need bounded dedicated workers before network integration.
## What Changes
Add CompilePool and asynchronous result handles. Bound worker count and waiting queue; reject saturation without blocking and preserve complete service results.
## Capabilities
### Modified Capabilities
- `service-api`: bounded asynchronous compilation dispatch.
## Impact
B0; crates/pp-cube-service, Cargo.lock and this change's OpenSpec artifacts/main service-api spec only. No snapshots, vendor or harness edits. No gRPC, SQL execution, dynamic model registration, cancellation or forced termination. Seed B0-API-004 remains proposed unchanged until network integration.
