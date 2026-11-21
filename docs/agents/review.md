# Evidence-grounded review

## Scope and design

Harness issue #33: reviewers previously received AGENTS.md and a diff, but not
complete archived artifacts, deleted base content or relevant public types.
Inference from roadmap language and rename similarity produced unsupported
findings. This change improves the supplied evidence and review continuity;
it cannot prove the semantic correctness of a model's conclusion.

Keep the reviewer without tools. Supply a packet from the exact PR base/head Git
objects: complete changed files on both sides, agent workflow/verification
contracts, active or archived change artifacts, seed transfer policy, traceability
script and public crate roots. Include the pinned diff. Deleted files exist only
on the base side. Preserve whitespace for line citations. Vendor source remains
excluded. Packets above 500 KB fail explicitly rather than silently truncating.
An omitted implementation dependency can still require context clarification.

The other choices were changing the reviewer alone (leaves missing evidence) or
granting repository tools (widens permissions and makes inputs less reproducible).
The selected design keeps the existing read-only boundary and delivery gates.

## Review contract

- Findings describe introduced defects and consequences, with exact citations:
  `evidence: [{revision: "base" | "head", path, line, quote}]`.
- Each finding location must itself have a citation. The controller checks the
  quote against that line of a supplied pinned file. This verifies provenance,
  not whether the text logically supports the claim. A matching quote alone is
  insufficient for a sound review.
- Missing context goes into `questions`, an array of strings. Any question blocks
  delivery without invoking code repair or consuming its repair rounds.
- Every previous finding has a zero-based `reassessments` entry with `index`,
  `status` (`resolved`, `withdrawn`, `remaining`) and a concrete `reason`.
  Remaining defects must also be reported in findings. New defects are allowed.
- History is retained only for the same repository, PR, branch and base, and an
  ancestor head. It is evidence to reassess, never authority to accept a claim.
- Protocol or reviewer-command changes invalidate the cached result. New reports
  remain bound to exact base/head/model; stale or malformed responses cannot pass.

The local review-packet.json preserves supplied file evidence. The PR comment
preserves the structured report. Both are outside tracked files, so recording
successful checks does not invalidate their SHA. Findings and context questions
both keep the PR Draft. Local CI, cross-vendor review, remote t0 and owner merge
remain required; no gate or approval rule changes.

## Model selection and verification

Default working arrangement for this effort: Codex / OpenAI GPT-6 implements;
omp / Anthropic Claude Opus 5.5 reviews with explicit model selection and
`--print --no-tools`. Configure per worktree; no credentials belong in Git.
Selecting Opus does not establish greater accuracy without comparative evidence.

Offline regressions exercise actual Git objects for archived/deleted context,
exact citations, prior-finding reassessment and questions blocking repair.
Run `python3 -m unittest discover -s scripts/tests`, then the standard fast/ci
checks. Real model and GitHub delivery evidence belongs in the PR at its SHA.

## Handoff

Implemented on harness/review-evidence, based on main after PR #29. Product PR #34
remains separate and Draft; do not merge or modify it as part of this effort.
Offline implementation and verification are complete before checkpoint. Real
Opus review and exact-head CI/t0 acceptance are pending publication; their results
and any bounded repairs are recorded in this PR rather than evidence-only commits.
Owner performs final review and merge. No production, vendor or OpenSpec changes.

First live Opus review returned a report but citation validation rejected its
line references. Add explicit per-line numbers in the model packet and retain
review-response.txt even for invalid responses, alongside the exact packet.
These are diagnostic artifacts, never acceptance evidence. The 500 KB cap also
covers the final numbered packet plus previous report. This is the first repair.


Second repair from a valid Opus report: non-ancestor/missing old heads now discard
history, test classes precede the direct-run entry point, and delivery.md links
this contract. To resolve context questions, rerun `scripts/dev review` or
`scripts/dev deliver` with repeatable `--review-context path/to/tracked/file`.
The named files are read from both pinned revisions, never the filesystem;
unknown paths and vendor files are rejected. The selected path set participates
in cache identity, so a changed selection triggers a fresh review on the same
head. Supply the same flags when resuming. If new repository evidence is needed,
commit it normally; a genuine owner-only decision follows the decision-issue rule.
Do not delete acceptance evidence or edit cached reports to clear questions.
Repeated reviews on the same SHA with different context produce distinct report
comment markers, so the PR retains the new verdict instead of hiding it behind
an earlier comment for that SHA.

## Bounded delivery stop

Stopped after two different review repairs. At cf1036b, Opus returned valid
citations and explicitly marked all three prior findings resolved. It reported
new issues: P2 base-side full files use the main tip while the three-dot diff uses
the merge base; P3 strict text decoding prevents review of non-UTF-8 files.
These are remaining implementation issues, not a service failure or ungrounded
policy inference. PR #35 stays Draft; do not bypass the gate or mark Ready.

Resume with a diverged-history regression and make the packet distinguish the
PR base tip from the diff pre-image, using consistent revisions for citations.
Add binary/non-UTF-8 coverage and a documented explicit handling path that does
not silently omit changed content. Keep all work in harness paths. Existing
issue #33 remains open until this PR is accepted. No product PR changes or merge.

Latest reviewed-head evidence: 27 offline tests passed;
`check ci: PASSED — 15 steps, 0 skipped, 44s @ cf1036b`.
Remote t0 passed on that head. T1/T2 not run. This final handoff-only commit will
receive its own CI evidence in PR comments; it is not a reviewed/accepted head.
The original automatic Opus route failed a service preflight; the explicitly
selected alternative Opus 5.5 route returned real reports. No credentials or
service configuration changes were needed.
