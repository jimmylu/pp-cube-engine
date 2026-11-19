# Proposal

## Why
Every other crate needs the contract types, and `pp-cube-model` is an empty stub. Model loading is the smallest B0 slice that depends on nothing else: no planner, no dialect, and none of the six questions the seed design leaves open.

## What Changes
- Add contract types for cube, dimension, measure, join and segment to `pp-cube-model`: plain data that is serialisable and shareable across threads.
- Load a model from YAML or equivalent JSON into the same types. A field or type value outside the baseline lists fails to load.
- Parse the three member reference forms and resolve them against a loaded model.
- Move two requirements, with their ids, out of `seed-baseline-specs` into this change: "Dimension and measure fields" (`B0-MODEL-001`) and "Joins, segments and references" (`B0-MODEL-004`).
- Baseline level B0. No SQL is produced, so no output and no golden snapshot changes.

## Capabilities
### New Capabilities
- `contract-model`: the Cube-compatible model fields the kernel loads and the reference forms it resolves. The other two requirements of this capability stay in the seed until validation and compilation exist.
### Modified Capabilities
None; no main specs exist yet.

## Impact
- `crates/pp-cube-model/**`: new code and tests.
- `Cargo.lock`: `serde`, `serde_json`, the YAML parser `serde_norway` and the JSON parser `serde_json_lenient` become dependencies of `pp-cube-model`; three packages are new to the lockfile, and no existing package's features change. The gate labels any lockfile change `risk:upstream`; the upstream pin does not move.
- `openspec/changes/seed-baseline-specs/specs/contract-model/spec.md`: the two moved requirements are deleted, as the seed design prescribes.
- On archive, `openspec/specs/contract-model/spec.md` becomes the first main spec and the traceability step starts enforcing its two scenarios.

## Out of Scope
Located validation issues (`B0-MODEL-002`, `B0-VALID-*`), model bundles with version and digest (`B0-MODEL-003`), query types, the adapter, dialects, the service, harness paths and `vendor/`.
