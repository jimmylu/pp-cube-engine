# Proposal

## Why

After dimension joins, the facade can compile B0 queries but cannot receive caller-supplied row restrictions. Adding security filters now completes the existing filter path before service exposure, without introducing authorization decisions.

## What Changes

- Add context security filters, appended beside user filters as top-level AND entries.
- Fail compilation for unusable restrictions, including empty groups; never retry without a restriction.
- Verify that cube SQL retains its mandatory predicate.
- Transfer the complete `security-filters` seed delta here, retaining B0-SEC-001 through B0-SEC-003; add B0-SEC-004 for empty groups.
- **BREAKING**: adding a public `CompileContext` field requires Rust struct-literal callers to supply an empty list when no restrictions apply.

## Capabilities

### New Capabilities

- `security-filters`: caller restrictions remain effective independently of user grouping, or compilation fails.

### Modified Capabilities

None. Existing query filter and join requirements remain in force.

## Impact

Planning scope was `openspec/changes/add-security-filters/**` and transfer of `openspec/changes/seed-baseline-specs/specs/security-filters/spec.md`. Completion moves the artifacts to `openspec/changes/archive/2026-11-20-add-security-filters/**` and creates `openspec/specs/security-filters/spec.md` in this same PR, as required by `docs/agents/workflow.md`.

Implementation scope: `crates/pp-cube-compile/src/lib.rs`, its tests, fixtures and new snapshots; narrowly scoped adapter filter validation if required by a reproduced failure. Existing compile tests need the empty context field. No new dependency is expected.

Baseline: B0, initially the available Presto dialect. Nonempty security filters add predicates and possibly declared dimension joins and parameters; empty context filters preserve existing SQL and parameter ordering. Existing snapshots must remain unchanged.

## Out of Scope

Authorization policy, credentials, SQL execution, service endpoints, model validation overhaul, complete result metadata, new error classes, Doris/Druid support, B1, issues #22/#23, vendor edits and harness changes. The original planning session changed documents only; the subsequent apply request authorized implementation and the normal completion workflow.
