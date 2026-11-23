# Model version lifecycle

## Why

The service could hold only the one model version given at startup, and its
"digest" was an unverified caller string. A deployed kernel has to take new
model versions without a restart, keep serving the versions callers still use,
and say verifiably which content a version is.

## What Changes

- **BREAKING** The model digest is computed by the service from the typed
  content (`sha256:` and 64 hex digits), independent of YAML or JSON formatting.
  It is no longer supplied by the caller. ValidateModel returns it for a valid
  model, and a publisher may state the digest it expects.
- Versions can be published, listed and retired at run time through three new
  gRPC operations. A version is immutable: the same content again is accepted
  and changes nothing; different content is refused. Retiring a version does not
  break compilations that already selected it. Other versions are never affected.
- Management is opt-in: the three operations are refused unless the host
  enables them (`--allow-publish` on the executable).
- **BREAKING** The executable takes repeatable `--model version=path` (several
  versions at startup) and drops `--version` and `--digest`.
- Baseline level: B0. No compile output or golden changes.

Non-goals: authentication and TLS (the next change; until then management is
only as safe as the loopback socket), persistence of published versions across
restarts, fan-out of a publish to several instances, a per-thread planner
cache keyed by digest, B1, new dialects.

## Capabilities

### New Capabilities

None.

### Modified Capabilities
- `contract-model`: the catalog lists and retires versions (B0-MODEL-009).
- `service-api`: version lifecycle (B0-API-013), content digest (B0-API-014), opt-in management (B0-API-015); the process requirement is updated.

## Impact

Paths: `crates/pp-cube-model/src/bundle.rs` and its test, `crates/pp-cube-compile`
(exports `model_digest`), `crates/pp-cube-service` (proto, core, gRPC,
executable, README, tests), the two specs and this change folder.
`CubeService::register` becomes `publish(&self, version, model, expected_digest)`.
