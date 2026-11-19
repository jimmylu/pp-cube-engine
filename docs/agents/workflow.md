# Change workflow

Every piece of work is a *change* with an id in kebab-case, for example `add-b0-aggregation`. One change is in implementation at a time.

Spec tooling (OpenSpec) is not installed yet. Until it is, the only permitted work is harness work on `harness/<topic>` branches. This document is completed when OpenSpec is set up.

## Lifecycle

| Stage | What happens | Gate |
| --- | --- | --- |
| Propose | A pull request containing only the change folder: proposal, spec delta, design, tasks. The proposal states what is out of scope. A model from the other vendor reads it before the owner does. | Gate 1: the owner approves `owner-approval` |
| Apply | Implementation pull requests on `change/<change-id>`. Each names the change id and the scenario ids it covers. | — |
| Review | A model from a different vendor than the implementer's reviews and posts a report as a comment. Fixes go back to the implementer. | Gate 2: `t0` green, the owner approves `owner-approval` |
| Archive | In the last implementation pull request, the spec delta is merged into `openspec/specs/`, so the specs on `main` describe what is built. | — |

No code for a change is written before its proposal has merged.

## Pull requests

- The merge gate is two required checks: `t0` (`scripts/check.sh ci`) and `owner-approval` (the owner approves it on GitHub). Every push needs a new approval.
- Fill in the evidence section of the pull request template. The summary line comes from `scripts/check.sh ci`, run on the commit you pushed.
- Files under `.github/workflows/` are pushed by the owner; agent tokens are refused. Prepare the change and hand it over.
- Squash merge only; history on `main` is linear.

## Handing a change to another agent

Hand over at a commit, with `tasks.md` and `notes.md` current. The next agent starts from those two files, not from a transcript.
