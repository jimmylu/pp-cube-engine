# Tasks

## 1. Transfer and cleanup

- [x] 1.1 Remove the temporal execution requirement from the seed Doris delta and record the transfer in the seed notes; verify `openspec validate --all --strict` passes and B0-DORIS-010 appears in exactly one delta.
- [x] 1.2 Name B0-DORIS-010 on the conformance Doris cases and delete the transitional opt-in test and `doris_time_engine.py`; verify `cargo test -p pp-cube-compile --test doris_time` and `cargo test -p pp-cube-conformance` pass with no pending snapshots.

## 2. Acceptance

- [x] 2.1 Run `scripts/check.sh doris-container` on real local Docker; verify the last line reports 13 live-compiled manifest cases and record it in notes.md.
- [x] 2.2 Run `scripts/check.sh ci` and `scripts/check.sh engines-container`; record each last line in notes.md.

## Workflow follow-up

- Sync and archive the delta before final review, in the same Draft PR.
- A later harness pull request updates the handoff sentence in `docs/agents/doris-container.md`.
