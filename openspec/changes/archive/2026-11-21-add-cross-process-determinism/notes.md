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
