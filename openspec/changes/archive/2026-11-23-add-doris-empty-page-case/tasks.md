# Tasks

## 1. Case

- [x] 1.1 Declare `page_past_last_group` for B0-DORIS-011, extend the golden and the manifest test; verify `cargo test -p pp-cube-conformance` passes with `INSTA_UPDATE=no`, the snapshot has forty blocks and no pending snapshot exists.
- [x] 1.2 Run `scripts/check.sh doris-container`, `engines-container` and `ci`; record each last line in notes.md.

## Workflow follow-up

- Sync and archive the delta before final review, in the same Draft PR.
- Then merge main into `harness/doris-manifest-only` (#67) and deliver it.
