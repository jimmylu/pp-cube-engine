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
