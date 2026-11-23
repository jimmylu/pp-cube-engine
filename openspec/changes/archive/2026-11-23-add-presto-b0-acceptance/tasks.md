# Tasks

## 1. Shared cases and Presto

- [x] 1.1 Split the cases into shared declarations with Doris fixtures and scenarios; verify the Doris cases are byte-identical before and after and `cargo test -p pp-cube-conformance` passes.
- [x] 1.2 Add Presto fixtures, the declared session property and the Presto golden with its test (B0-EXEC-004, B0-PRESTO-017); verify forty blocks, `INSTA_UPDATE=no` passes and no pending snapshot exists.
- [x] 1.3 Execute the Presto manifest on real local Presto 0.293 on demand; verify forty cases pass with the property and the two paginated cases fail without it; record both in notes.md.

## 2. Rules and specs

- [x] 2.1 Update `openspec/config.yaml` to the work-sized change rules; verify `openspec validate --all --strict` passes.
- [x] 2.2 Reword B0-PRESTO-003 in the seed to literal expected rows and record the transfer plan; verify traceability passes.
- [x] 2.3 Run `scripts/check.sh ci`; record the last line in notes.md.

## Workflow follow-up

- Sync and archive both deltas before final review, in the same Draft PR.
- Harness pull request: the Presto tier compiles and executes this manifest with its session properties (closes #61).
- Transfer B0-PRESTO-003 out of the seed once that tier has passed on GitHub.
