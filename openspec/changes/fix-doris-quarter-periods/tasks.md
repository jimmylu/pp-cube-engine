# Tasks

- [x] 1 Run the 85 cases on Doris 2.1.8 one by one, and by hand the date functions no case covers; record the table in design.md.
- [x] 2 Write quarters as months in the two Doris templates; test B1-DORIS-005; review the two goldens that change.
- [x] 3 Run the 85 cases on Doris 2.1.8 again: 85 pass.
- [x] 4 Run `doris-container` (4.1.3); record the line in notes.md. `conformance` not run: no Presto path changes.
- [ ] 5 Run `scripts/check.sh ci`; record the last line in notes.md.

## Workflow follow-up

- Sync and archive the delta before final review, in the same Draft PR.
- Then the harness change: tiers on Presto 0.296 and Doris 2.1.8.
