# Prepared models, measurement and packaging

## Why

Nobody had measured a compilation. Measuring showed that every compilation
validated and digested the whole model again: at 50 cubes that was 95% of an
11.5 ms compilation. The service also had no image and no guide for a user.

## What Changes

- `PreparedModel` validates and digests a model once and compiles any number of
  queries against it. `compile` keeps its behaviour and is now a one-off
  prepare-and-compile. Results are identical, fingerprint included.
- The service prepares a version when it is published, so a compilation no
  longer validates or digests. Measured at 50 cubes: p50 11.5 ms to 0.64 ms.
- `compile-bench` measures latency per phase and multi-thread throughput for
  the shared cases on a model of any size; `docs/performance.md` records the
  method and the figures.
- A `Dockerfile` builds a non-root, shell-less service image, and
  `docs/guide.md` explains how to use the library, the service and a result.
- Baseline level: B0. No compile output or golden changes.

Non-goals: the per-thread planner context cache of the original design (the
measurement shows it is not the bottleneck; what remains is about 10 µs per
cube and unprofiled); publishing images or releases, version numbering and
aarch64 builds (harness paths, the next pull request); B1; new dialects.

## Capabilities

### New Capabilities

None.

### Modified Capabilities
- `compile-result`: adds the prepared model requirement (B0-RESULT-004).

## Impact

Paths: `crates/pp-cube-compile` (facade, fingerprint internals, test),
`crates/pp-cube-service/src/lib.rs`, `crates/pp-cube-conformance` (padded model
helpers, new benchmark binary), `Dockerfile`, `.dockerignore`, `docs/guide.md`,
`docs/performance.md`, `README.md`, the spec and this change folder.
