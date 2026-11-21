# Session notes

## 2026-11-21: next PR planned

PR #28 merged as main 0673ee4b7eece4f453e6df17d32e68f13b8d96ce. Started
change/add-classified-compile-errors from that exact base with a clean tree.
Implementer/planner: Codex / OpenAI GPT-6.

Prepared proposal, design, spec delta and five bounded implementation tasks.
Transferred the complete Classified errors / B0-RESULT-002 requirement verbatim
from the seed; its now-empty compile-result delta was deleted as specified by
seed design's transfer policy. Main specs are unchanged; no implementation,
archive, new dependency, vendor or harness change has occurred.

Observed: typed facade has only Model/Query/Internal; PlanError has User/Internal.
Joins::check detects direct and dependency-derived multi-owner measure requests,
but reports a user error. Public compile validates the whole model first.
Existing Query decoding rejects unknown fields before compile; raw parse/error
APIs remain outside this PR. Existing tests cover multi-owner rejection as Query
and must change only for recognized same-source out-of-B0 requests.

Selected typed error propagation rather than matching upstream message text or
duplicating dependency analysis in the facade. Physical schema/SQL must stay out
of caller messages; internal diagnostic details remain operator-only. No owner
question or extra scope is currently required.

Open: all five implementation tasks, their tests, safe-message audit, full CI,
archive and exact-head final delivery. Proposal status is not implemented support.
The Draft PR remains the single vehicle for planning, implementation and archive.
No final acceptance or T1/T2 execution is claimed. Verification/publication results
are recorded below as available and in SHA-bound PR evidence after checkpoint.

Planning verification: `openspec validate add-classified-compile-errors --strict`
passed; `openspec validate --all --strict` passed 9 items; `git diff --check`
passed. Checkpoint runs fast before commit. Full CI and cross-vendor product
review are not run at the planning checkpoint because implementation is pending.


## Implementation checkpoint

Tasks 1.1–2.2 implemented. Added Unsupported at the dependency-aware join check
and facade conversion; join/data-source faults precede unsupported classification.
Local entry-point checks retain logical diagnostics. Unknown upstream user errors
receive a safe rule-only message; internal detail is operator-only. Audited join
callback failures are retained in a per-compilation receipt, and forwarded only
when the planner propagates the exact original failure. This does not infer a
failure class from a message substring or trust arbitrary upstream diagnostics.

B0-RESULT-002 tests cover all four classes, physical SQL/schema markers, invalid
model/query/security precedence, multi-source and disconnected/ambiguous cases,
security-only multi-owner measures and existing formula dependencies. Internal
mapping is tested at the actual adapter and facade conversion seams, with no
public injection API. Existing golden SQL/parameter snapshots are unchanged.

Red: new Unsupported assertion failed to compile against the former enum.
Green: `cargo test -p pp-cube-adapter -p pp-cube-compile --locked` passed.
Full CI, archive and final exact-head acceptance remain pending. No new dependency,
vendor or harness edits. Scope remains one classified-error behavior.


## Pre-review completion

All five implementation tasks complete. Full CI passed:
`check ci: PASSED — 15 steps, 0 skipped, 98s @ ed90a0f+dirty`.
This describes the pre-publication working tree, not accepted final-head evidence.
Strict validation passed 9 items. Source/test changes stay inside the stated
adapter/facade scope; existing golden snapshots, Cargo.lock, vendor and harness
are unchanged. Size reviewed: retain this one classified-error behavior together.

The completed requirement is synced into the existing compile-result main spec
without changing its successful-result requirement or Purpose, and this folder
is archived before final review. Final exact-head CI, cross-vendor review and
remote t0 remain pending; their results belong in PR comments/artifacts. Owner
reviews and merges after Ready. T1/T2 not run. No known implementation blocker.


## First review repair

Review on 0d1be73 requested three corrections, now implemented:
- Facade Internal.detail no longer retains raw planner SQL or physical identifiers;
  both Debug and Display are covered by the sentinel assertion. Adapter errors
  remain internal diagnostics. This tightens implementation to the unchanged spec.
- Upstream user mapping includes logical selected measures/dimensions and the
  failing planner stage, without disclosing the untrusted upstream text.
- Replaced text-equality receipts with the first typed local callback failure,
  checked at every planner stage even if upstream wraps/suppresses its error.
  A real join-bridge rejection test covers both wrapped failure and success.

`cargo test -p pp-cube-adapter -p pp-cube-compile --locked` passed after repairs.
Final-head CI/review/t0 remain pending. No spec weakening, dependency, harness,
vendor or golden snapshot edits. Original checkpoint descriptions above are
historical; this section supersedes their diagnostic-retention mechanism.
