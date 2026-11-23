# Design

## Context

See proposal.md. Third unit of the productization route. The layer split is
kept: the library offers what any host needs (access, limits, telemetry,
health); the executable owns the listener, TLS, files and signals.

## Decisions

- **Bearer tokens in two roles, checked in the service.** Read admits Compile,
  ValidateModel and GetMeta; admin admits everything. Checked per operation
  inside the handlers, because the role depends on the operation, and compared
  without an early exit. With no token every caller is trusted, which the
  executable only allows on loopback. Alternatives: mTLS (stronger, but needs a
  certificate authority the deployment may not have; left as a later option) and
  an external gateway only (leaves management open to anything that reaches the
  port). A read token without an admin token denies management outright rather
  than letting readers publish.
- **Fail closed at startup.** Beyond loopback needs TLS and a read token; tokens
  with management need an admin token; a token is at least 16 visible ASCII
  characters. Tokens are read from files so they never appear in a process list.
- **TLS through tonic's rustls with ring.** Both crates were already resolved in
  the lockfile, so no licence decision was needed. TLS 1.2 and 1.3 with rustls
  defaults. A server certificate must be an end-entity certificate; rustls
  rejects a CA certificate presented by a server.
- **Standard health protocol, compiled from a small proto here.** Probes work
  without knowing this service's contract and without the `tonic-health` crate.
  Watch is unimplemented, which the protocol defines as "assume healthy".
  Health is unauthenticated and reveals only serving or not.
- **Telemetry records the shape of a request, never its content.** Fields are
  operation, outcome, duration, request id, model version, a known dialect name
  and the result fingerprint. Metric labels come from fixed sets, so cardinality
  is bounded. Caller text is cut to 128 characters and JSON-escaped. An
  unauthenticated request is still counted and logged, with what it claimed.
- **Metrics over a minimal HTTP responder.** `GET /metrics` on its own address,
  nothing else, with a size and time bound per connection. A full HTTP stack
  for one fixed response was not worth a dependency edge; the endpoint carries
  no secrets but should stay on an internal address.
- **A deadline, not a cancellation.** The planner is synchronous and cannot be
  interrupted, so a caller stops waiting after the deadline while the worker
  finishes. The bounded queue still protects the process: when workers are held
  by slow compilations, new requests get Busy.
- **Drain.** A signal clears the serving flag, stops accepting and lets running
  requests finish; after the drain time the process exits unsuccessfully.

## Risks / Trade-offs

- Static tokens need a restart to rotate and identify roles, not callers →
  stated; a gateway or mTLS can be layered on by a host using the library.
- The drain-timeout exit path and non-Unix signal handling are not covered by a
  test.
- The metrics responder is hand-written HTTP → it answers one path with a fixed
  shape and is covered by a process test.
- The process tests call `openssl` → present on the CI image and macOS.

## Migration Plan

Additive for a loopback deployment: with no new option the process behaves as
before, plus logs on standard error (`--log off` disables them) and the limits'
defaults. Revert to remove. Evidence: `scripts/check.sh ci`, remote `t0`,
`presto-smoke`, `doris-smoke`.
