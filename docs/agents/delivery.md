# Automatic delivery

User-authorized work continues through planning, implementation, repair and publication.
Only owner decisions and final approval/merge require intervention. A normal change uses
one Draft PR; its proposal need not merge separately. Harness paths remain isolated.

## Start and resume

Run `scripts/dev status` at session start and after publication. It reports branch, exact
head, dirty state, pushed head and PR identity; network failure is a blocker, never a
fallback to reviewing an unrelated branch. Work on one approved change per worktree.
A meaningful slice that passes fast is committed and pushed immediately:

```sh
scripts/dev checkpoint --message 'change-id: complete one verified slice'
scripts/dev deliver
```

Checkpoint maintains one Draft PR per branch. It never force-pushes or merges. Provide
explicit, real model identities and command arrays once per worktree before publication:

```sh
scripts/dev configure \
  --implementer 'anthropic/your-configured-model' \
  --implementer-command '["omp","--print","--approval-mode","write"]' \
  --reviewer 'openai/your-configured-model' \
  --reviewer-command '["codex","exec","--sandbox","read-only","-"]'
```

Identities must match the models actually selected in the tools' profiles. These labels
are provenance declarations, not provider attestation. Use explicit tool model flags if
profiles change. Reverse the vendors when OpenAI implements. For omp review use
`--print --no-tools`; review receives contract documents and the pinned diff on stdin.
Repair prompts leave edits uncommitted; the controller checks and commits them.
Do not grant a reviewer write tools. Model credentials remain in the tools' own profiles;
GitHub publication tokens are removed from model process environments.

`deliver --rounds 2` checks local CI, gets cross-vendor review, publishes its SHA-bound
report, repairs findings, checkpoints, and repeats. Exhaustion leaves a Draft PR and a
visible blocker. A model failure or invalid reply cannot become a passing review.
Remote t0 must pass for the exact reviewed head before Ready. It never waits for owner
approval itself. The final gate runs only for a Ready PR. Owner approval stays in the
existing protected environment; switching its authorization mechanism is a separate
administrative migration, not a prerequisite to automated development.

A lock under the worktree's Git directory prevents simultaneous controllers. A crash can
leave it behind: inspect its PID before removing it. Resume the same command afterward;
`deliver` checkpoints pending edits with `fix: resume pending delivery changes` after
the usual scope validation and fast check. PR discovery prevents duplicate creation.
Report SHA, push status, PR URL and blockers
after every publication. CI/review evidence lives outside tracked files; do not amend a
commit just to record the successful check of that same commit.

## Issues and boundaries

Routine check/review findings are fixed within the PR. Create an issue only for a separate
bug, a cross-change dependency, an after-baseline request, or an owner-only decision;
reuse the existing issue for the same blocker. Do not weaken gates to get a passing run.
Around 400 lines prompts scope review; keep independently verifiable behavior and fixes
together. Archive only implemented, meaningfully tested scenarios in the final PR.

Workflow changes require a publication credential with workflow write permission.
The owner bootstraps that identity once; it must not bypass main protection or approve
its own environment gate. Until provisioned, workflow publication is a reported blocker.
No credentials or private endpoint names belong in the repository.

## Verification and design sources

`python3 -m unittest discover -s scripts/tests` runs offline controller regressions and
is part of ci. Final end-to-end delivery additionally requires configured model tools,
GitHub access, workflow publication permission and a real Draft PR; report those checks
as not run if unavailable. Simulated tests do not prove external service integration.

- CLI fields and command behavior: local `gh pr view --help`, `codex exec --help`, `omp --help`.
- GitHub gate definitions are trusted base-branch code and never execute PR code:
  https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#pull_request_target

## Initial rollout record

Implemented by Codex (OpenAI GPT-6) on harness/dev-delivery. The traceability lifecycle
fix already arrived through #14 and was retained. Eleven offline regressions and local
fast/ci passed. A real Anthropic review was attempted through omp but could not start:
no Anthropic API credential is available. End-to-end model repair and GitHub Ready
transition remain unverified. Keep the rollout PR Draft until another vendor reviews it.
The existing environment approval is retained, enabled only for Ready PRs; repository
rules and publication credentials were not altered. Workflow publication may still need
the owner's existing workflow-capable identity. Product code was not changed.

Subsequent real review succeeded through omp's configured DeepSeek route and posted a
SHA-bound report to #16. Fixed latest-t0 selection, explicit pending-snapshot rejection,
and documentation discovery. The unpushed-branch report was disproved by the existing
empty-output guard and a new regression. A full model-driven repair cycle is still
unverified; repairs in this rollout were made by the implementing agent.

Review repair session on harness/dev-delivery (left uncommitted at the user's request):
the reported missing Ready trigger was already present on `pull_request_target`; a
regression now covers it without changing the gate. Both model roles now drop all four
GitHub publication token variables, including enterprise tokens. Resumed delivery can
checkpoint edits left by an interrupted repair before continuing CI/review. Three new
offline regressions cover these findings; existing tests remain. External delivery and
publication were not run in this session. No owner decision is open.
Verification: all 18 controller tests passed. Check summaries for this uncommitted
worktree (not evidence for a published head):

```text
check fast: PASSED — 4 steps, 0 skipped, 2s @ bc905a5+dirty
check ci: PASSED — 15 steps, 0 skipped, 21s @ bc905a5+dirty
```
