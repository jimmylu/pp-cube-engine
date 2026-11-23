# Deployable and operable service

## Why

The service could only run for trusted callers on a loopback socket: no
encryption, no caller identity, no probe, no log, no metric, no bound on what a
request may cost, and it ignored the signal orchestrators stop a process with.
Model management, added in the previous change, was guarded only by being off.

## What Changes

- **Access tokens.** A read token and an admin token, presented as bearer
  credentials. Management needs the admin token whenever tokens are in use.
- **Encrypted listening.** The executable accepts a TLS certificate and key and
  listens beyond loopback only with TLS and a read token. Tokens come from files.
- **Health.** The standard `grpc.health.v1` Check, unauthenticated, reporting
  not serving once the host drains.
- **Telemetry.** One JSON log line and one count per finished request, with a
  duration histogram; never SQL, values, tables, model content or credentials.
  Optional Prometheus text endpoint on a separate address.
- **Bounds.** A compilation deadline, a request size limit and a limit on
  available versions.
- **Drain.** SIGTERM and SIGINT stop accepting, let running requests finish
  within a drain time and exit.
- Baseline level: B0. No compile output or golden changes.

Non-goals: client certificates (mTLS), token rotation without restart, more
than two roles or per-caller identity, tracing, persistence of published
versions, cancelling a running compilation (the planner cannot be interrupted),
B1, new dialects.

## Capabilities

### New Capabilities

None.

### Modified Capabilities
- `service-api`: access tokens (B0-API-016), health (B0-API-017), telemetry (B0-API-018), bounded requests (B0-API-019), deployable host (B0-API-020); the process requirement no longer demands loopback.

## Impact

Paths: `crates/pp-cube-service` (Cargo features, build script, health proto,
gRPC layer, new telemetry module, executable, README, tests),
`crates/pp-cube-model/src/bundle.rs` (one error variant), `Cargo.lock` (two
dependency edges, no new package), the spec and this change folder. tonic gains
the `tls-ring` feature; rustls and ring were already in the lockfile, so the
licence policy is unchanged. The process tests need `openssl` on the path.
