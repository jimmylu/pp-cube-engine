# Proposal

## Why
The seed still holds the upstream-pin contract even though fingerprint identity already embeds the pin. The repository test checks the gitlink, but does not connect the actual checkout to a public compile result.

## What Changes
- Adopt Pinned unmodified planner / B0-UPSTREAM-001 verbatim from the seed.
- Extend its repository test to compare gitlink, actual checkout HEAD and the pin used in a real compilation's fingerprint, including pin-change sensitivity.
- No observable production or SQL change is expected.

## Capabilities
### New Capabilities
- `upstream-pin`: repository pin and compiled-result identity agreement.
### Modified Capabilities
None.

## Impact
B0 only. Scope: test code in crates/pp-cube-compile/src/fingerprint.rs, this folder, openspec/changes/seed-baseline-specs/specs/upstream-pin/spec.md for transfer, and eventual openspec/specs/upstream-pin/spec.md. No vendor edits, pin upgrade, production behavior, dependencies, snapshots or harness changes. Bundle identity APIs and dialect work remain separate.
