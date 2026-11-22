# Tasks

Implementation is complete, the main specs are synced, and this change is archived.
Review corrections continue within this change's scope. Each task is at most two hours.

## 1. Typed boundary

- [x] 1.1 Add failing load/round-trip tests for all B0-QUERY-011 members and unknown/malformed controls; add typed optional fields and granularity variants with API documentation; verify model tests and workspace compilation after updating struct literals.
- [x] 1.2 Replace old hour load-error expectations while retaining B0-QUERY-008 unknown-name coverage; verify model and compile time tests still check the five supported units and all date operators.

## 2. Compilation classification

- [x] 2.1 Add model-first public compile tests for every recognized boundary input (B0-QUERY-009); add the shared boundary predicate and facade guard; verify invalid models return Model and valid models return Unsupported without results.
- [x] 2.2 Guard direct adapter planning and test the complete B0-AGG-003 matrix: multi-cube measures, ungrouped true, comparison presence (empty and populated), second/minute/hour; verify exact error classes and false-grouped success, without silently dropping options.
- [x] 2.3 Add serialization/fingerprint regression assertions for absent fields and SQL/parameter equality for explicit false; verify existing fingerprint tests, golden tests and four-process replay without snapshot changes.

## 3. Integration

- [x] 3.1 Verify scope and scenario matrix against the delta, run strict OpenSpec validation and scripts/check.sh ci; record results and source-compatibility changes in notes.md and the Draft PR.

## Workflow follow-up

- [x] Verify implemented behavior, sync the two deltas and archive in this same PR before final review; no T1/T2 claims.
- [x] Fix review findings about the unknown-granularity diagnostic, API documentation and archived lifecycle record; these corrections were subsequently committed and published in PR #40.
- [x] Clarify B0-AGG-003 multi-cube validation precedence in the main spec and archived delta, and distinguish historical publication restrictions from the current repair request.
- PR #40 contains the published implementation and earlier corrections. This repair session leaves its new edits uncommitted in the worktree, without pushing, merging or altering gates. That session-specific restriction does not prohibit a subsequent authorized delivery controller from checking, committing and publishing repairs under `docs/agents/delivery.md`.
- Delivery must establish different-vendor review and exact-head remote t0 evidence for the corrected head; final approval and merge remain with the owner.
