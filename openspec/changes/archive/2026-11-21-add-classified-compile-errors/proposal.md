# Proposal

## Why

The facade exposes Model, Query and Internal failures but cannot identify a
recognized feature outside B0. Callers need a reliable failure class and safe
messages before adding more dialects or a service boundary.

## What Changes

- **BREAKING**: add Unsupported to CompileError and the adapter's typed errors;
  exhaustive matches must handle the new variant.
- Classify multi-fact measure requests as unsupported without enabling B1 SQL.
- Keep malformed queries and invalid models in their existing classes, and keep
  non-user planner failures internal with a generic public message.
- Transfer the complete Classified errors requirement and B0-RESULT-002 from
  seed-baseline-specs; remove its now-empty compile-result delta.
- Preserve successful SQL, parameters, metadata and fingerprints.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- compile-result: add the proposed Classified errors requirement to the existing
  successful-result contract, preserving its stable scenario id.

## Impact

B0 only. Scope: crates/pp-cube-adapter/src and its tests;
crates/pp-cube-compile/src and tests/fixtures; this change folder;
openspec/changes/seed-baseline-specs/specs/compile-result/spec.md for the complete
requirement transfer; openspec/specs/compile-result/spec.md at archive.
No new dependencies are expected. Existing success snapshots must stay unchanged.

Non-goals: raw JSON query parsing APIs, recognizing additional Cube JSON fields,
new dialects, B1 compilation, service/wire errors, runtime IO, vendor or harness
changes. Model validation continues returning the existing issue report.
