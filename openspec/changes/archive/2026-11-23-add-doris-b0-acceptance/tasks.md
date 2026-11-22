# Tasks

## 1. Cases and goldens

- [x] 1.1 Extend the conformance model and declare the twenty-six matrix cases with one retail fixture and literal expectations; verify the emitter prints thirty-nine cases and the eleven temporal SQL statements equal their existing goldens.
- [x] 1.2 Add the golden of every Doris case (B0-DORIS-001) and update the manifest test for B0-EXEC-001; verify `cargo test -p pp-cube-conformance` passes with `INSTA_UPDATE=no` and no pending snapshot exists.

## 2. Acceptance

- [x] 2.1 Run `scripts/check.sh doris-container` on real local Docker; verify the last line reports thirty-nine live-compiled manifest cases and record it in notes.md.
- [x] 2.2 Transfer B0-DORIS-001 and B0-DORIS-002 out of the seed, delete the emptied seed Doris delta and record the transfer; verify `openspec validate --all --strict` and `scripts/traceability.sh` pass.
- [x] 2.3 Run `scripts/check.sh ci` and `scripts/check.sh engines-container`; record each last line in notes.md.

## Workflow follow-up

- Sync and archive both deltas before final review, in the same Draft PR.
- Harness pull request: remove the stored-golden assertions and fixture from the Doris tier and update `docs/agents/doris-container.md`.
