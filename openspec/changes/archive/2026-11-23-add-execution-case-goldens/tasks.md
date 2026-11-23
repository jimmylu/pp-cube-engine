# Tasks

## 1. Golden

- [x] 1.1 Add `insta` to the conformance dev-dependencies and the golden test for B0-EXEC-003; verify `cargo test -p pp-cube-conformance` passes with `INSTA_UPDATE=no`, the snapshot holds thirteen blocks and no pending snapshot exists.
- [x] 1.2 Run `scripts/check.sh ci`, `engines-container` and `doris-container`; record each last line in notes.md.

## Workflow follow-up

- Sync and archive the delta before final review, in the same Draft PR.
- Then merge main into `change/add-doris-b0-acceptance` (#64) and deliver it.
