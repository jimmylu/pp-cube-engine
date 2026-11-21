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
