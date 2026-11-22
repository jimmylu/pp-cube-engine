# Session notes

## Pre-review completion

Change: add-cross-process-determinism, based on main e6836b8 / merged PR #29.
Implementer: Codex / OpenAI GPT-6; reviewer: omp / DeepSeek V4 Pro, pending.
Deferred work from #29 is in issues #30–#33, all labelled after-baseline.

All three implementation tasks complete. The aggregation, filters, joins, time
and security suites import a shared checked compile wrapper. Every successful
input is replayed in four fresh processes with the same model/query/context,
including request_id. All complete Compiled fields are compared against the
parent via derived Debug records. Workers invoke production compile directly;
no recursion or shared temporary files. Failed compiles do not spawn workers.
Malformed worker input must exit nonzero and emit no result; comparison mutation
tests cover SQL, parameters, columns, used cubes and fingerprint independently.
Inline and loop-generated golden cases use the same wrapper automatically.

`cargo test -p pp-cube-compile --locked` passed. Pre-publication CI:
`check ci: PASSED — 15 steps, 0 skipped, 93s @ e6836b8+dirty`.
This is working-tree evidence, not final accepted head evidence. Existing golden
snapshots, production code, dependencies, vendor and harness are unchanged.
No upstream nondeterminism observed. Scope is one golden determinism behavior.

The Repeatable results requirement and B0-DET-001 moved verbatim from seed into
this change, then synced to determinism main specs preserving Fingerprint identity
and Purpose. Archive before final review is required by docs/agents/workflow.md
Lifecycle / Archive. Final accepted exact-head CI, cross-vendor review and remote
t0 remain pending, recorded in PR comments/artifacts rather than another commit.
Owner reviews and merges after Ready. T1/T2 not run; no known blocker.


## Review evidence: baseline enum and approved transfer policy

Review at c6430ea reported a speculative E0004 and a transfer-scope concern.
The exact base e6836b81e980f8de30c81a398fe0ee710282d4a4 contains this entire
public enum in crates/pp-cube-dialect/src/lib.rs:

```rust
pub enum DialectId {
    Presto,
}
```

Doris/Druid are roadmap targets, not variants of the current typed API. The
single-arm match is intentionally exhaustive so adding a future dialect forces
this test transport to be updated. Production fingerprint.rs uses the same
exhaustive Presto match. This PR compiled locally and on remote t0 at c6430ea:
`check ci: PASSED — 15 steps, 0 skipped, 68s @ c6430ea`.
Do not add a wildcard that hides future variants to fix a nonexistent E0004.

seed-baseline-specs/design.md explicitly defines the authorized transfer rule:
"Transfer: before an implementing change adopts a requirement, it moves that
complete requirement and its ids out of this seed in the same reviewed update.
Its proposal names the seed spec file in its scope, so deleting the moved text
is inside that change. Never keep duplicate ADDED requirements in both changes."

This proposal's Impact names that exact seed path, and the complete Repeatable
results / B0-DET-001 block moved verbatim. This is one implementing change's
stated scope, not implementation of an unrelated change. No other seed behavior
is changed. docs/agents/workflow.md requires final archive in the same PR before
final review. Neither rule requires moving the artifacts back or weakening gates.

The first review remains failing pending fresh cross-vendor reassessment of this
evidence. This checkpoint is documentation only; final-head acceptance is pending.

## Review follow-up: direct scenario traceability

The ecb996d reassessment no longer reports the enum or transfer findings. Its
remaining P2 asks for B0-DET-001 on the golden tests themselves rather than only
on shared support. Added the scenario comment directly to all 28 successful
golden-suite tests, preserving existing scenario ids. This includes the time-order
regression with both a successful compile and a refused query. No behavior or
snapshots changed. Local CI passed on ecb996d:
`check ci: PASSED — 15 steps, 0 skipped, 72s @ ecb996d`; remote t0 also passed.
This is the second review repair; final-head acceptance remains pending in PR #34.
If the gate still fails after this repair, stop and record the blocker rather
than starting an unbounded repair loop. Final evidence stays in PR comments.

## Delivery stopped after two repairs

Final reassessment of 55d06662c6b8a56b554a5db7fb618f2381b483ac still
reports findings. Stop condition reached after two different review repairs.
PR #34 remains Draft; do not bypass review or mark Ready. No production defect
has been demonstrated. Implementation and archive are complete; acceptance is open.
Latest exact reviewed-head local evidence:
`check ci: PASSED — 15 steps, 0 skipped, 80s @ 55d0666`.
Remote t0 also passed on that head. T1/T2 were not run.

The final P1 infers from Git rename similarity that the seed determinism file
also contained Fingerprint identity / B0-DET-002. Reading that entire file at
base e6836b81e980f8de30c81a398fe0ee710282d4a4 proves it contains only
Purpose, ADDED Requirements, and Repeatable results / B0-DET-001.
B0-DET-002 is already in main specs and remains there. Similarity is not evidence
of another deleted requirement. The P2 claims a one-test-one-scenario convention;
AGENTS.md rule 4 requires a test naming each implemented scenario, without that
restriction. scripts/traceability.sh compares sets of ids and imposes no such
restriction; docs/agents/verification.md describes the same set comparison.

Record these review-context limitations in existing deferred issue #33 rather
than modifying harness code in this product PR. Resume only when the review
context/gate concern can be addressed within a separately scoped effort or an
owner decision. This handoff-only commit changes no tests or behavior; its check
evidence is recorded separately in the PR, bound to the resulting SHA.


## Resumed after merged harness correction

The owner requested completion through Ready. Harness PR #35 is merged as
68fe053cc97efc53f59e6264dbfdf0558156aa17. Synchronize that main tip by a normal
merge (no history rewrite or force push), then use the merged evidence-grounded
review controller with omp / Anthropic Claude Opus 5.5. Implementer remains
Codex / OpenAI GPT-6. Previous bounded stop is historical, not an approval bypass.

All implementation tasks remain complete. No new product behavior, snapshots,
vendor changes or PR-specific harness changes are needed. The merge imports
already merged harness history; relative to main this PR retains its original
test/spec scope. Fast runs before the merge commit. Exact-head CI, Opus review
and remote t0 are pending; final evidence will be posted to PR #34 without a
further evidence-only commit. Owner reviews and merges after Ready. T1/T2 not run.


## Opus worker transport follow-up

Opus reported a progress-prefix collision and requested the other test binaries
to verify golden coverage. Reproduced with RUST_TEST_THREADS=1 on aggregation's
one_sum_grouped_by_one_dimension: the child prints its record after libtest's
progress prefix, so no tagged line is found. Worker now emits a leading newline
and always selects --test-threads=1. All five golden suites pass with
RUST_TEST_THREADS=1; their normal runs also continuously exercise this child mode.
No production output or snapshots change. Local CI before repair passed:
`check ci: PASSED — 15 steps, 0 skipped, 74s @ 765d85b`.

Repository-wide search for insta snapshot assertions finds only aggregation,
filters, joins, time and security. Provide all other compile integration tests
(result.rs, errors.rs, model_validation.rs) as supplemental context for Opus;
result.rs has direct assertions and a focused fingerprint replay, not goldens.
Expanding the single-thread run to every compile binary exposed the pre-existing
fingerprint child prefix collision in result.rs; that full command failed.
This independent B0-DET-002 transport issue is deferred in #36 (after-baseline),
not hidden as passing evidence and not part of the new golden worker. Default
full CI and the five-suite single-thread run remain the acceptance checks here.
Final exact-head acceptance remains pending in PR comments.
