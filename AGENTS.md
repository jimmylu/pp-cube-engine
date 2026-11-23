# pp-cube-engine

A Rust compile kernel: Cube data model + query + context in, SQL + parameters out, for Presto, Doris and Druid. It compiles; it never executes SQL. Planning is done by upstream `cubesqlplanner`, vendored unmodified.

This repository is public. Commits, pull requests, issues and notes carry no internal system names, hostnames, credentials or business data. Fixtures use neutral names.

## Check command

```sh
scripts/check.sh fast   # before every commit
scripts/check.sh ci     # before requesting final review; CI runs exactly this as `t0`
```

Use `scripts/dev checkpoint --message "..."` to check, commit, push and maintain a Draft PR.
Use `scripts/dev deliver` to run local CI, wait for the remote checks (`t0` and, when the change can affect engines, the engine containers) and request owner review. It runs no model and starts no container.
Use `scripts/dev review` for a cross-model review only when the owner asks for one; it never runs automatically.
Report commit SHA, push status, PR URL and blockers after every publication.
The last check line is evidence; store evidence against its SHA in PR comments or CI artifacts. A check you did not run is reported as not run.

First use in a fresh clone: `scripts/vendor-sync.sh` (a plain `git submodule update` downloads all of Cube).

## Hard rules

1. **Work inside one approved change.** The branch is `change/<change-id>` and the pull request title starts with the change id. Harness work uses `harness/<topic>`.
2. **`vendor/` is read-only.** Needed upstream behaviour is handled in `crates/pp-cube-adapter` or reported.
3. **Golden snapshots change only when the change's spec delta says the output changes.** Review with `cargo insta review`; pending snapshots are never committed.
4. **Every scenario you implement gets a test naming its id**: `// scenario: B0-AGG-003`.
5. **Harness changes travel alone.** A pull request touching a harness path (`.github/`, `scripts/`, `AGENTS.md`, `CLAUDE.md`, `.agents/`, `.claude/`, `.codex/`, `.omp/`, `docs/agents/`, lint and toolchain configuration) touches nothing else.
6. **Stay inside the current baseline level.** Anything beyond it becomes an issue labelled `after-baseline`.
7. **The owner merges.** Automate delivery through Ready; owner reviews the final head and merges.
8. **A change is one capability, sized by work.** Each pull request delivers one independently usable capability. Plan a change as about 2–3 working days of work, up to 4–5 when the capability needs it. There is no line or file limit; do not split a capability to stay small.
9. **Test and framework optimization travels in its own batch.** A capability pull request carries the tests its capability needs and no more. Reworking existing test cases, fixtures or the test framework is collected and done once, in a dedicated pull request, not spread across capability pull requests or repeated.

## Stop conditions

Stop, write the state into the change's `notes.md`, and report when:

- the same check fails twice after two different fixes;
- the work needs a file outside the change's stated scope, or a harness path;
- upstream behaviour contradicts the spec;
- the change runs past its planned size (rule 8) by about a working day without a reviewable result;
- a question only the owner can answer comes up: open an issue labelled `decision`.

Stopping with a clear report is a successful outcome.

## Session protocol

- **Start:** run `openspec list`, then read the active change's `tasks.md` and `notes.md`.
- **End:** update `tasks.md`; write what was done, what is open and what was learned into `notes.md`. Everything needed to continue lives in the repository, so any agent can pick the change up at a commit.

## Roles

One agent at a time owns a change's branch. Review by a second model is on demand: the owner starts it, or asks for it, when a change warrants one. When it runs, the reviewing model comes from a different vendor than the implementing model, posts its report as a pull request comment and pushes nothing. The pull request records which tool and model implemented, and the reviewer only when a review ran.

## Read when

| Document | Read it when |
| --- | --- |
| `docs/agents/delivery.md` | committing, publishing, reviewing or resuming delivery |
| `docs/agents/workflow.md` | starting, proposing, implementing, reviewing or archiving a change |
| `docs/agents/verification.md` | a check fails, a tool is missing, or you need a tier beyond `ci` |
| `docs/agents/skills.md` | choosing a skill for a task |
| `docs/agents/glossary.md` | a project term is unclear (B0, B1, facade, adapter, golden, scenario) |
| `docs/agents/launch.md` | setting up or starting an agent session (tokens, local guard) |
| `vendor/AGENTS.md` | reading upstream code or anything about the upstream pin |
