# Session notes

## Current state

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
error behavior together. No new owner decision or implementation blocker exists.
The next step is checkpoint, exact-head CI, cross-vendor reassessment and remote
t0. Evidence belongs in PR comments/artifacts, avoiding an unreviewed evidence
commit. A successful checkpoint is not a claim of final accepted CI/review.
