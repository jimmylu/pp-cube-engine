# Tasks

## 1. Establish

- [x] 1.1 Put rolling-window, to-date and time-shift queries to Cube Core 1.8.28 and to the kernel; record the table in design.md. Verified by comparing SQL and parameters of 53 queries.
- [x] 1.2 Check Presto 0.293's interval literals and month arithmetic on a container; record in design.md.

## 2. Model

- [x] 2.1 Add `rolling_window`, `multi_stage` and `time_shift` to the measure contract, unserialized when absent; test B1-MODEL-001.
- [x] 2.2 Validate their form and combinations; test B1-VALID-001 and B1-VALID-002; update B0-VALID-003; verify `cargo test -p pp-cube-model`.

## 3. Compile

- [x] 3.1 Pass the fields through the measure bridge; hand date ranges over as Cube's API layer does; verify no existing golden changes.
- [x] 3.2 Interval functions and time-series templates for Presto; dialect interval units; test B1-ROLL-001 to -004 and B1-SHIFT-001, -002 with goldens.
- [x] 3.3 Refuse windows and shifts without their time dimension (B1-WIN-001) and what Presto cannot express (B1-PRESTO-001).
- [x] 3.4 Doris templates with an enumerated series; test B1-DORIS-002 with goldens.

## 4. Evidence

- [x] 4.1 Add window and shift measures and nine execution cases for both dialects; verify existing cases' SQL is unchanged.
- [ ] 4.2 Run `conformance` and `doris-container` locally; record the lines in notes.md.
- [ ] 4.3 Update `README.md`, `docs/guide.md`, `CHANGELOG.md`; verify `openspec validate --all --strict` and traceability.
- [ ] 4.4 Run `scripts/check.sh ci`; record the last line in notes.md.

## Workflow follow-up

- Sync and archive the deltas before final review, in the same Draft PR.
- Next unit of the route: B1 views, ungrouped queries, approximate distinct, custom granularities.
