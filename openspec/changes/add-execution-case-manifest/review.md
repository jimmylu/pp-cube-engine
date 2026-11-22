# B0 progress and route

Assessment of main `f7e8729` on 2026-11-22, replacing the route in the archived
`add-query-boundary-classification/review.md`.

## Evidence held

Contract, validation, query boundary, compile result, security filters,
fingerprints, cross-process determinism and the gRPC service have T0 evidence.
Presto compiles the B0 matrix (B0-PRESTO-001, 004 to 016). Doris compiles basic,
joined, pattern and temporal queries (B0-DORIS-003 to 009). Smoke execution:
three stored Presto goldens; nineteen stored-golden Doris assertions; thirteen
live-compiled Doris temporal cases through an explicit opt-in command.

## Open, all still proposed in seed-baseline-specs

| Scenario | Missing |
| --- | --- |
| B0-DORIS-010 | Temporal cases in the defined execution tier (issue #59) |
| B0-DORIS-001 | Audit and complete the Doris matrix goldens |
| B0-DORIS-002, B0-PRESTO-003 | Full T2 from live-compiled cases |
| B0-PRESTO-002 | T1 Cube Core comparison; the tier is not built |
| B0-DRUID-001 to 004 | Druid dialect, declared join boundary, T1 and T2 |

## Structural findings

1. Execution tiers parse stored snapshots and Debug text. Doris: this change and
   issue #59. Presto: a cross-change harness dependency that needs its own issue.
2. No independent semantic oracle exists until T1; goldens show stable output.
   Covered by B0-PRESTO-002 and B0-DRUID-003, so no separate issue.
3. Druid, part of the largest stated unknown, is untouched. Covered by
   B0-DRUID-001 to 004, so no separate issue.

## Route

1. Doris accepted: this manifest; harness tier integration (#59); matrix
   completion and full T2 (B0-DORIS-010, 001, 002).
2. Presto T2 on the same manifest (B0-PRESTO-003).
3. Druid compilation and declared boundary (B0-DRUID-001, 002).
4. T1 for Presto and Druid, and Druid T2 (B0-PRESTO-002, B0-DRUID-003, 004).
5. Delete the seed folder, T3 by a person, declare B0, then propose B1.

Step 3 precedes step 4 to surface dialect risk early and build T1 once for two
dialects; the owner may move Presto T1 ahead of Druid. Issues #22, #23, #30,
#31, #32 and #36 stay deferred as `after-baseline`.
