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

Read [review.md](review.md) for the evidence, history and context-question contract.
Use `scripts/dev deliver --review-context path/to/tracked/file` to supplement missing
context without changing harness code; repeat the flag for additional files.

`scripts/dev deliver` is deliberately light. It checkpoints pending edits, runs
`scripts/check.sh ci` locally, then waits for the remote checks on the exact head:
`t0` always, `image` and `aarch64` when the diff touches a path that starts the
`build` workflow (see [verification.md](verification.md)), and `presto-smoke`,
`doris-smoke` and `cube-core` when the diff touches a path that starts the
`engines` workflow (crates, templates, engine scripts, `scripts/check.sh`,
`scripts/vendor-sync.sh`, the Cargo manifests and toolchain, the workflow itself).
When they pass it marks the PR Ready and posts an evidence comment that says what
ran and what did not. A failing local or remote check leaves the PR Draft; fix it
and run `deliver` again. It never waits for owner approval itself.

Delivery runs no model and starts no container (owner decision, 2026-11-23):

- **Cross-model review is on demand.** Run `scripts/dev review` only when the owner
  asks for one. It posts a SHA-bound report; findings are fixed in the session and
  re-checked by another `review` only if the owner wants that. There is no
  automatic repair loop. A review that exists for the delivered head is mentioned
  in the evidence comment; none is required for Ready.
- **Engine containers run on GitHub.** The `engines` workflow executes the Presto
  smoke and the live-compiled Doris manifest for every pull request that can affect
  them, and `deliver` requires both to pass when they are triggered. Run
  `scripts/check.sh engines-container` or `doris-container` locally when debugging
  an engine failure or developing engine cases; they are not part of default checks.

Idempotent GitHub reads, fetches and pushes are retried twice after a dropped
connection. Owner approval stays in the existing protected environment.

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
Size a change by work, not lines: one capability, about 2–3 working days, up to 4–5
when needed (AGENTS.md rule 8). Collect test-case and test-framework rework into one
dedicated pull request instead of repeating it across capability changes (rule 9). Archive only implemented, meaningfully tested scenarios in the final PR.

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

## Rollout state

History up to 2026-11-22. Review and repair rounds described below were removed
from delivery on 2026-11-23; the section above is current.

The owner authorized all three delivery phases, including removal of the gate's t0
polling. The controller requires successful local CI, cross-vendor review and remote
t0 before marking Ready. If a human marks Ready directly, owner approval may be
requested before t0 finishes; the independently required t0 check still blocks merge.
This ordering is intentional. The existing owner-approval environment is retained;
repository protection, credentials and environment configuration were not changed.
Because pull_request_target uses trusted base code, #16 uses the previous gate until
this workflow change is merged.

Implemented with Codex / OpenAI GPT-6; reviewed through omp / DeepSeek V4 Pro.
Real checkpoint publication, review comments and two bounded model-driven repair
rounds ran on #16. Repairs were automatically checked, committed and pushed as
410fd2e and c71907b. All 21 offline controller regressions passed. Local CI and remote
t0 passed at c71907b; its final review found only contradictory rollout notes, now
replaced by this record. Final SHA-bound review and Ready evidence belong in the PR,
not in a new commit that would invalidate that evidence.

No product files or vendor source changed. Harness work has no OpenSpec task folder;
this document and PR #16 hold its handoff state. No owner-only decision is open. The
owner performs final approval and merge.
