# Design

## Context
See proposal.md for scope. The workspace is scaffolded; no product behavior is implemented. The public design records prior experiments, not test results from this repository.

## Goals / Non-Goals
Create reviewable B0 acceptance targets. Do not implement all twelve capabilities as one change or archive untested promises.

## Decisions
- Use twelve capability deltas matching the engineering plan, rather than a monolithic spec. Stable ids survive transfer into later approved implementation changes.
- Owner decision #5 (2026-11-19): complete bootstrap step 5 with proposed specs. The alternative, archiving now with a traceability exception for unimplemented scenarios, was rejected because it weakens the gate.
- Specs state their own lists (fields, operators, granularities, the B0 feature matrix) instead of citing design section numbers, so editing the draft design cannot change a requirement without a delta.
- Transfer: before an implementing change adopts a requirement, it moves that complete requirement and its ids out of this seed in the same reviewed update. Its proposal names the seed spec file in its scope, so deleting the moved text is inside that change. Never keep duplicate ADDED requirements in both changes. A capability file that loses its last requirement is deleted; the pull request that moves the last requirement deletes this folder, because strict validation rejects a change without deltas (checked with OpenSpec 2.1.0).
- Splitting: an implementing change may split a matrix scenario. The original id stays with one part or is retired; new parts take the next free number in the area; no id is reassigned.
- Tiers: a scenario titled `(T1)` or `(T2)` needs execution (`docs/agents/verification.md`); all others are T0. Its test lives in `pp-cube-conformance`, names the id and is not run by T0. A requirement holding such a scenario is archived only after its tier is built and has passed for it; an unexecuted test is not coverage.
- `upstream-pin` keeps only what a kernel test can observe. Checkout integrity and toolchain and `[patch]` alignment stay with the `vendor` and `upstream` steps of `scripts/check.sh`; the upgrade procedure stays in kernel design 6.9 and `vendor/AGENTS.md`.
- Publish prerequisites separately: pending harness commit; public design contract chapters; remaining kernel chapters and config; finally this change-folder-only proposal. Keep each authored PR below approximately 400 lines.

### Choices beyond the source draft, for the owner to confirm at gate 1
- The fingerprint excludes the request id. Design 3.4 hashes the whole compile context; 6.7 uses the fingerprint for plan caching, which a per-request value defeats.
- Druid declares joins unsupported and returns unsupported. Section 8 accepts only single-cube queries on Druid and 6.3 forbids speculative SQL.
- Any second eligible join path is a query error. This is stricter than the shortest-path wording in 6.2 and never picks a path silently.
- Relative date ranges and queries spanning data sources are query errors.
- Security filters fail closed: one that cannot be applied fails the compilation.

## Risks / Trade-offs
- Coarse matrix scenarios → a test claims a matrix scenario only when it exercises every member the scenario names; an id comment alone is insufficient.
- Translation of historical measurements → retain their limitations; do not present default-template SQL as validated Presto, Doris or Druid output.
- Not fixed by the source draft, and not chosen here → implementation proposals decide before writing code: query/context timezone precedence; service wire parameter typing; cache isolation; how a bundle version reaches the service (6.8 has no load operation) and the failure class for an unavailable version; who verifies the bundle digest; how data_source maps to a dialect and what happens when the context dialect disagrees.
- Upstream contradictions → stop and report under AGENTS.md; do not patch vendor or silently revise expected behavior.

## Migration Plan
Documentation only; no runtime migration. Revert an unapproved draft to undo it. Validation: strict OpenSpec validation, an audit of unique scenario ids, and reviewers comparing the public edition with source chapters 1–8 and appendix A while excluding private integration material.
