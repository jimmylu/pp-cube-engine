# Change workflow

Every piece of work is a *change* with an id in kebab-case, for example `add-b0-aggregation`. One change is in implementation at a time. Changes are managed with OpenSpec; the folder `openspec/changes/<change-id>/` is the record.

Harness work is the exception: it has no change folder and lives on `harness/<topic>` branches.

## What a change folder holds

| File | Content |
| --- | --- |
| `proposal.md` | What and why; what is out of scope |
| `specs/<capability>/spec.md` | The spec delta: `ADDED`, `MODIFIED`, `REMOVED` or `RENAMED` requirements, each with scenarios |
| `design.md` | How |
| `tasks.md` | The task list of record |
| `notes.md` | Ours, not OpenSpec's: what was done, what is open, what was learned. Updated at the end of every session |

`openspec status --change <change-id>` shows which artifacts exist. `openspec/config.yaml` holds the rules each artifact must follow.

## Lifecycle

| Stage | Skill | What happens | Gate |
| --- | --- | --- | --- |
| Explore | `openspec-explore` | Optional: think a problem through before proposing | — |
| Propose | `openspec-propose` | Create the change folder on `change/<change-id>` and continue in the same Draft PR | Agent verifies scope; ask owner only for a decision |
| Apply | `openspec-apply-change` | Implement in that Draft PR; name the change id and scenario ids covered | — |
| Review | `openspec-verify-change`, then the reviewer | A model from a different vendor than the implementer's reviews and posts a report as a comment. Fixes go back to the implementer | `t0` green, then final owner approval |
| Archive | `openspec-archive-change` | Before final review in the same Draft PR: sync the implemented spec delta into `openspec/specs/` and the folder moves to `openspec/changes/archive/` | — |

A user-authorized ordinary change carries proposal, implementation and final archive in one Draft PR. Separate proposal approval is needed only for unresolved owner decisions. After archiving, main specs describe implemented behavior.

## Scenarios

Each scenario title starts with a stable id, `<LEVEL>-<AREA>-<NUMBER>`: `#### Scenario: B0-AGG-003 Sum over one cube`. An id is never reused or renumbered. Once a spec is in `openspec/specs/`, every scenario in it needs a test naming its id, or `scripts/check.sh ci` fails; a spec delta inside a change folder is not checked this way until it is archived. A test may name a scenario of an active change's delta before the archive, so the Draft PR records the ids implemented; an id found in no spec and no active delta fails the check.

## Pull requests

- The merge gate is two required checks: `t0` (`scripts/check.sh ci`) and `owner-approval` (the owner approves it on GitHub). Only Ready PRs request final owner approval; push fixes while Draft. Evidence and review bind to the exact head SHA.
- Fill in the evidence section of the pull request template. The summary line comes from `scripts/check.sh ci`, run on the commit you pushed. The controller posts evidence outside Git, so recording a result does not change its commit.
- Publishing `.github/workflows/` requires a workflow-capable identity. Use the configured identity; report a credential blocker if unavailable. Final owner approval remains required.
- Squash merge only; history on `main` is linear.

## Handing a change to another agent

Hand over at a commit, with `tasks.md` and `notes.md` current. The next agent starts from those two files, not from a transcript.

## Automatic publication

Read delivery.md before starting a delivery session. A complete verified slice is committed
and pushed with `scripts/dev checkpoint --message "..."`; this creates or updates one Draft PR.
`scripts/dev deliver` runs CI, another vendor's review and at most two repair rounds, then
marks it Ready. The owner approves and merges. Report SHA, push status and PR URL each time.
Around 400 lines is a scope review prompt, never a reason to leave known correctness bugs behind.
Create an issue for an independent defect, cross-change dependency, after-baseline work or
an owner-only decision. Fix routine test/review findings in the current PR without new issues.
Reviewers never edit the implementation worktree; the controller records their report on the PR.
