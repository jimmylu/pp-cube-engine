# Session notes

## Current state: stopped after bounded review repair

PR: https://github.com/jimmylu/pp-cube-engine/pull/29
Branch: change/add-classified-compile-errors
Implementer: Codex / OpenAI GPT-6. Reviewer: omp / DeepSeek V4 Pro.
All five implementation tasks are complete. Final accepted exact-head evidence
and cross-vendor review remain pending at this checkpoint. PR remains Draft;
owner approval and merge follow Ready. T1/T2 not run.

## Implemented behavior

The facade and adapter distinguish Unsupported from invalid-query failures at
the existing dependency-aware multi-owner measure check. Model validation, query
validation, data-source and join-path faults retain precedence. Security-only
measure dependencies and formulas participate. Success snapshots, fingerprints,
dependencies, vendor and harness are unchanged.

Public errors carry no raw planner SQL or physical source identifiers. Upstream
user failures receive logical selected members and the failing planner stage;
local checks retain their logical diagnostics. Adapter Internal retains operator
diagnostics, while the facade replaces raw detail with a safe summary. Tests use
physical schema/column/SQL sentinels at the real conversion seams. No injection API.

## Review repairs and verification

0d1be73 review: scrubbed facade Internal.detail, supplied member/stage context for
upstream user errors, and removed the original exact-message-equality mechanism.
The first typed local callback rejection now wins, and finish() checks it after
every upstream stage including successful returns. Adapter/facade tests passed.
Full local CI and remote t0 passed at b32c6a7; its review remained failing.

b32c6a7 review: removed the unused From<CubeError> implementation completely,
including its generic-message path. Future fallible CubeError calls must now
explicitly select local or context-aware upstream conversion to compile.

The callback-state tradeoff is intentional: pinned upstream bridge trait methods
return Result<_, CubeError>, so returning PlanError explicitly through them would
require a forbidden vendor/interface change. Joins is allocated once per plan(),
shared synchronously with that invocation's planner, and discarded afterward.
Its first local validation rejection is fatal even when upstream wraps or
suppresses it; later errors cannot make that request valid. finish() consumes the
rejection at each stage, independently of upstream cause/message. Existing tests
exercise actual bridge rejection followed by wrapped failure and success. This
retains typed provenance without parsing messages or maintaining global state.

Archive timing is required by docs/agents/workflow.md, Lifecycle / Archive:
"Before final review in the same Draft PR". The Classified errors requirement
and B0-RESULT-002 were transferred verbatim from the seed and synced to the
existing compile-result spec; Complete compile result and Purpose are preserved.
The branch id and scripts/dev status identify this archived in-flight change.
It must not be moved back to active merely because the owner has not merged.

Scope was reviewed above 400 lines: keep this independently testable classified
error behavior together. No owner-only design decision has been identified; final delivery remains gated by review.
The next step is checkpoint, exact-head CI, cross-vendor reassessment and remote
t0. Evidence belongs in PR comments/artifacts, avoiding an unreviewed evidence
commit. A successful checkpoint is not a claim of final accepted CI/review.


## Final diagnostic repair

49e502a review requested all query-member contexts and concrete upstream rules.
Mapping now includes nested filter, segment, time and order members. Audited
pinned-upstream depth/date/precision/parameter templates map to fixed safe rules.
A real 170-member filter dependency chain triggers the planner's 160-level
resolution guard through compile(); the test verifies its member/rule and no
physical identifiers. Debug frames require a bounded 32 MiB test thread stack.
The existing facade Internal.detail field is retained for source compatibility
and documented as a safe category rather than a diagnostic record.
Final CI, cross-vendor review and Ready evidence remain pending at checkpoint.


## Bounded review stop after 325adbf

Stopped under AGENTS.md after the review gate remained failing following two
distinct repair attempts (49e502a and 325adbf). No more product edits are made in
this handoff. PR #29 remains Draft; Ready and owner review have not been requested.

Exact-head evidence on 325adbf12fb90271e1ebc6dab03b63dd3cd3509e:
`check ci: PASSED — 15 steps, 0 skipped, 46s @ 325adbf`.
Remote t0 passed on that same head. Cross-vendor review failed. This handoff only
receives checkpoint fast unless a later full CI result explicitly says otherwise.
T1/T2 were not run. No harness, vendor, dependency or existing golden edits.

Remaining findings from the SHA-bound report:
- P2: Reviewer again asks to confirm archive-before-final-review. This is explicitly
  required by docs/agents/workflow.md Lifecycle / Archive and already quoted above.
  Do not unarchive or change the harness to accommodate this misunderstanding;
  supply the authoritative workflow evidence for reassessment.
- P2: Verify the pinned planner does not recover from a speculative build() failure
  before accepting the first typed local_failure latch as fatal. Trace all calls
  to Joins::try_build/build_join through Tools and pinned planner callers; add a
  regression or redesign provenance if recovery is possible. No unsupported claim
  about upstream recovery behavior is established yet.
- P3: The real dependency-depth failure test uses a 32 MiB thread stack. Review
  questions portability to other toolchains/profiles. Find an equally meaningful
  bounded trigger or justify the pinned-toolchain stack requirement; do not remove
  all actual upstream-rule coverage or weaken the spec just to pass review.
- P3: Public Internal.detail now carries a safe category instead of raw operator
  diagnostics. Add that semantic compatibility change to proposal/PR BREAKING
  notes, not only design and API comments.

All implementation tasks have their exercised verification, but final acceptance
is incomplete. Next authorized resume should address the findings in this same
PR, checkpoint, and re-run final exact-head CI/review/t0 within a fresh bound.
