# Proposal

## Why
The remaining seed Cube fields requirement has no direct acceptance test for every source-selection failure. Close this B0 model-entry contract before expanding dialects or service behavior.

## What Changes
- Transfer Cube fields / B0-MODEL-002 verbatim from the seed.
- Test all four specified failure cases in both YAML and JSON, including exact issue paths and rules.
- Add B0-MODEL-006 for accepting all listed cube fields with either source and rejecting unknown cube fields.
- No observable runtime or SQL change is expected: the current validator already implements these rules.

## Capabilities
### New Capabilities
None.
### Modified Capabilities
- `contract-model`: adopt the remaining cube field/source contract, plus explicit accepted-field coverage.

## Impact
Scope: crates/pp-cube-model/tests/cube_sources.rs; this change folder; openspec/changes/seed-baseline-specs/specs/contract-model/spec.md for the transferred requirement; eventual sync to openspec/specs/contract-model/spec.md. No production, dependency, vendor, snapshot or harness changes. Bundle replacement, B1, service and deferred issues are non-goals.
