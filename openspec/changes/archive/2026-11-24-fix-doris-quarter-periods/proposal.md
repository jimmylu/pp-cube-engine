# Doris writes quarters as months

## Why

Production runs Doris 2.1.8; the tiers accepted Doris 4.1.3. Run on a Doris
2.1.8 frontend and backend, 84 of the 85 execution cases passed. The one that
failed, a granularity of two quarters, uses `QUARTER_FLOOR`, which 2.1.8 does
not have. Trying by hand the other date functions the dialect writes found a
second one no case covers: 2.1.8 refuses `INTERVAL 1 QUARTER` in `DATE_ADD`
and `DATE_SUB`. First part of route item G1 (`review.md` of
`archive/2026-11-24-add-insurance-scenario`).

## What Changes

- Doris counts periods of quarters with `MONTH_FLOOR` over three times as
  many months, and writes a quarter interval as three months. Before:
  `QUARTER_FLOOR` and `INTERVAL n QUARTER`, which run on 4.1.3 only.
- Two Doris goldens change in those expressions and nowhere else. Results are
  the same: the execution case for two quarters keeps its expectation.
- Baseline level: B1. Presto is untouched.

Non-goals: moving the tiers to the production versions, which is a harness
change and follows; Presto 0.296, where all 85 cases pass unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities
- `dialect-doris`: quarters as months (B1-DORIS-005).

## Impact

Paths: `templates/doris.yaml`, tests and two goldens of `pp-cube-compile` and
`pp-cube-conformance`, one spec and this change folder. Callers: Doris SQL
for quarter granularities, shifts and windows changes text, not results, and
now runs on Doris 2.1.
