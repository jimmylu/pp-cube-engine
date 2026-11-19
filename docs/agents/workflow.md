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
| Propose | `openspec-propose` | A pull request on `change/<change-id>` containing only the change folder. A model from the other vendor reads it before the owner does | Gate 1: the owner approves `owner-approval` |
| Apply | `openspec-apply-change` | Implementation pull requests on `change/<change-id>`. Each names the change id and the scenario ids it covers | — |
| Review | `openspec-verify-change`, then the reviewer | A model from a different vendor than the implementer's reviews and posts a report as a comment. Fixes go back to the implementer | Gate 2: `t0` green, the owner approves `owner-approval` |
| Archive | `openspec-archive-change` | In the last implementation pull request: the spec delta is merged into `openspec/specs/` and the folder moves to `openspec/changes/archive/` | — |

No code for a change is written before its proposal has merged. After archiving, the specs on `main` describe what is built.

## Scenarios

Each scenario title starts with a stable id, `<LEVEL>-<AREA>-<NUMBER>`: `#### Scenario: B0-AGG-003 Sum over one cube`. An id is never reused or renumbered. Once a spec is in `openspec/specs/`, every scenario in it needs a test naming its id, or `scripts/check.sh ci` fails; a spec delta inside a change folder is not checked this way until it is archived.

## Pull requests

- The merge gate is two required checks: `t0` (`scripts/check.sh ci`) and `owner-approval` (the owner approves it on GitHub). Every push needs a new approval.
- Fill in the evidence section of the pull request template. The summary line comes from `scripts/check.sh ci`, run on the commit you pushed.
- Files under `.github/workflows/` are pushed by the owner; agent tokens are refused. Prepare the change and hand it over.
- Squash merge only; history on `main` is linear.

## Handing a change to another agent

Hand over at a commit, with `tasks.md` and `notes.md` current. The next agent starts from those two files, not from a transcript.
