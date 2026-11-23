# Complete compile contract

## Why

The service contract was a prototype: parameters were untyped JSON strings,
lineage a flattened list of table names, nothing told a caller which engine
session settings the SQL needs (found on Presto: `OFFSET` fails without one),
nothing identified what produced a response, a request could name a dialect that
does not serve the model's data source, and metadata lacked what a modelling
interface shows. The wire format is cheapest to fix now, before any caller exists.

## What Changes

- **BREAKING** `ppcube.engine.v1` is rewritten in place (the owner confirmed
  there is no caller): typed parameters, structured used cubes with data source
  and table paths, a failure-kind enum, typed metadata messages, and an identity
  (kernel version, upstream pin, model version and digest) on Compile and GetMeta.
- The compile result declares the engine session properties its SQL needs.
  Presto declares `offset_clause_enabled=true` exactly when the query has an
  offset; the execution manifest now takes its session from compile results.
- The service maps each data source to a dialect. A model on an unmapped data
  source is not made available, and SQL is returned only when the requested
  dialect serves every data source the query reads. The executable takes
  repeatable `--data-source name=dialect`.
- GetMeta adds data source, segments, measure format, primary keys, `meta` as
  written, and joins to public cubes. It still exposes no SQL or tables.
- B0-PRESTO-003 leaves the seed: the Presto tier runs the manifest on GitHub (#72).
- README status and the design's stale proto listing are corrected.

Non-goals: model lifecycle beyond one startup version, TLS, authentication,
logging and metrics, limits (next changes); B1 features; new dialects.

## Capabilities

### New Capabilities

None.

### Modified Capabilities
- `compile-result`: adds declared session properties (B0-RESULT-003).
- `dialect-presto`: the offset property is declared by the compile result (B0-PRESTO-017); adds Presto execution acceptance (B0-PRESTO-003).
- `service-api`: typed wire contract (B0-API-010), data source dialects (B0-API-011), discoverable metadata (B0-API-012); metadata and process requirements updated.

## Impact

Paths: `crates/pp-cube-dialect`, `crates/pp-cube-compile`, `crates/pp-cube-model`
(one error variant), `crates/pp-cube-service` (proto, core, gRPC, executable,
README, tests), `crates/pp-cube-conformance` (session source), `README.md`,
`docs/design/engine-kernel.md`, `openspec/changes/seed-baseline-specs/`,
the three specs and this change folder. Baseline level: B0. Generated SQL and
every golden are unchanged. `Compiled` gains a field, `CubeService` is
constructed with data sources, and `render` in the conformance crate loses its
session argument.
