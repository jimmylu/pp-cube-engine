# Tasks

## 1. Establish

- [x] 1.1 Put multi-fact queries to Cube Core 1.8.28 and to the kernel without its refusals; record the table in design.md. Verified by comparing SQL and parameters of 49 queries.

## 2. Compile

- [x] 2.1 Add the four templates the planner asks for from the pin; verify a two-fact query renders on both dialects.
- [x] 2.2 Validate per fact instead of refusing several facts; one data source across facts; test B1-MULTI-001, -002, -004, -005 with goldens; verify `cargo test -p pp-cube-compile --test multi_fact`.
- [x] 2.3 Stop refusing multi-stage SQL; measures over repeated rows; test B1-MULTI-003.
- [x] 2.4 Refuse a filter on a count over repeated rows by name; test B1-MULTI-006.
- [x] 2.5 Lineage through key subqueries; test B1-MULTI-007.
- [x] 2.6 Doris goldens (B1-DORIS-001); update the tests whose expectation the delta changes (B0-AGG-003, B0-JOIN-006, B0-DORIS-006, B0-RESULT-002).

## 3. Evidence

- [x] 3.1 Add two fact tables and seven execution cases for both dialects; verify existing cases' SQL is unchanged.
- [ ] 3.2 Run `conformance` and `doris-container` locally; record the lines in notes.md.
- [ ] 3.3 Update `README.md`, `docs/guide.md`, `CHANGELOG.md`, `openspec/config.yaml`; verify `openspec validate --all --strict` and traceability.
- [ ] 3.4 Run `scripts/check.sh ci`; record the last line in notes.md.

## Workflow follow-up

- Sync and archive the deltas before final review, in the same Draft PR.
- File the filtered-count behaviour as an issue.
- Next unit of the route: B1 time shift, rolling window and period to date.
