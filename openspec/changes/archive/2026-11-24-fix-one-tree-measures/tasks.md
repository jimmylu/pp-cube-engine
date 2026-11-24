# Tasks

- [x] 1 Put thirteen queries around the defect to Cube Core 1.8.28 and to the kernel; record the table in design.md.
- [x] 2 Accept a fact without a tree of its own when one tree holds the whole query; test B1-MULTI-008; verify `cargo test -p pp-cube-compile --test multi_fact`.
- [x] 3 Move the two scenario queries to compiled in the matrix golden and add them as execution cases; verify existing cases' SQL is unchanged.
- [x] 4 Run `conformance` and `doris-container` locally; record the lines in notes.md.
- [x] 5 Update `README.md` and `docs/guide.md`; run `scripts/check.sh ci`; record the last line in notes.md.

## Workflow follow-up

- Sync and archive the delta before final review, in the same Draft PR. Close #88 on merge.
- Next on the route: G1, production engine versions.
