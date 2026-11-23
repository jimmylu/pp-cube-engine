# Tasks

## 1. Establish

- [x] 1.1 Put ungrouped, approximate-count and granularity queries to Cube Core 1.8.28 and to the kernel; record the table in design.md. Verified by comparing SQL and parameters of 25 queries.
- [x] 1.2 Read the planner's default origin; check Cube's origin expression on a Presto 0.293 container for rows before the origin; record both in design.md.

## 2. Contract

- [x] 2.1 `Granularity::Custom`, `granularities` on dimensions, `count_distinct_approx`; test B1-MODEL-002 and B1-QUERY-001; update B0-QUERY-008 and B0-QUERY-011.
- [x] 2.2 Validate granularities and refuse a window on an approximate count; test B1-VALID-003.

## 3. Compile

- [x] 3.1 Ungrouped: pass the flag, refuse measure filters and grouped parts; test B1-DETAIL-001 and -002; update B0-AGG-003 and B0-QUERY-009 tests.
- [x] 3.2 Approximate count as a dialect function; test B1-APPROX-001.
- [x] 3.3 Resolve declared granularities, `date_bin` as a dialect function, refuse undeclared names, rolling and ranges before the origin; test B1-GRAN-001 and -002.
- [x] 3.4 Doris templates; test B1-DORIS-003 with goldens.

## 4. Evidence

- [x] 4.1 Add six execution cases for both dialects; verify existing cases' SQL is unchanged.
- [x] 4.2 Run `conformance` and `doris-container` locally; record the lines in notes.md.
- [x] 4.3 Update `README.md`, `docs/guide.md`, `CHANGELOG.md`; verify `openspec validate --all --strict` and traceability.
- [x] 4.4 Run `scripts/check.sh ci`; record the last line in notes.md.

## Workflow follow-up

- Sync and archive the deltas before final review, in the same Draft PR.
- Views, the rest of unit 8, as the next change.
