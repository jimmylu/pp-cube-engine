# Design

## Established on Doris 2.1.8

A frontend and a backend of the published `fe-2.1.8` and `be-2.1.8` images,
reporting `doris-2.1.8-rc01`. Every execution case was run one by one.

| What | Doris 2.1.8 | Doris 4.1.3 |
| --- | --- | --- |
| 85 execution cases, before the change | 84 pass; `granularity_of_two_quarters` fails: no function `quarter_floor(datetimev2, tinyint, datetimev2)` | 85 pass |
| 85 execution cases, after the change | 85 pass | recorded in notes.md |
| `MONTH_FLOOR`, `WEEK_FLOOR`, `YEAR_FLOOR`, `DAY_FLOOR` with an origin | run | run |
| `DATE_ADD` and `DATE_SUB` with `INTERVAL n QUARTER` | refused: invalid time unit | run |
| `DATE_ADD` and `DATE_SUB` by week, month and year; `DATE_TRUNC` by quarter and week; `APPROX_COUNT_DISTINCT` | run | run |
| `MONTH_FLOOR` of a date before its origin | floors correctly (2025-06-15 with six months from 2026-01-01 gives 2025-01-01) | not examined |

## Decisions

- **One text for both versions.** A quarter is three months in the two Doris
  templates that count or move time; nothing branches on the engine version,
  which the kernel does not know. Alternative: a version setting per
  deployment; rejected as a second axis of output for no gain.
- **The second defect has a compile test, not an execution case.** Cases are
  shared with Presto, which has no quarter interval at all, so a shift of a
  quarter cannot be one. The expression was run by hand on 2.1.8.

## Risks / Trade-offs

- Month arithmetic clamps to the month's end, a quarter interval could in
  principle differ → three months and one quarter clamp alike; not examined
  beyond that.
- Doris 2.1.8 was provisioned by hand for this change; the tier still runs
  4.1.3 until the harness change that follows.

## Migration Plan

Text of Doris SQL changes for quarters; results do not. Evidence:
`scripts/check.sh ci`, remote `t0`, `doris-smoke`, `presto-smoke`, `cube-core`.
