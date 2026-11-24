# Property-insurance scenario

## Why

Every feature so far was chosen from Cube's list and tested on a small retail
model. The kernel is to serve property-insurance analysis, and no model or
query of that domain has been put to it; real ones do not exist yet. Until
one is, which refusals matter and which gaps block going live are guesses.
This is step 0 of the route in `review.md`, which this change carries.

## What Changes

- **A scenario model**, neutral and invented: policies, written and earned
  premium, exposures, claims with payments and reserves, products and lines
  of business, channels and intermediaries, regions, reinsurance cessions, and
  views that present them to analysts.
- **A set of business queries**: loss ratio, earned against written premium,
  claim frequency and severity, paid and outstanding by accident and report
  period, period to date and same period last year, renewal and retention,
  ceded share, with the filters, security filters and paging such reports use.
- **A support matrix**: every query compiled for Presto and Doris, recorded as
  compiled or refused with the refusal's reason, and each refusal classified
  as needed for the domain or not.
- **Execution cases** on a small fixture for the queries that compile, run on
  both engines and against Cube Core.
- No change to what compiles. Baseline level: B1. New goldens only.

Non-goals: fixing what the survey finds (each becomes a change or an issue,
ordered by the matrix); production engine versions; Druid; actuarial
correctness of the invented figures.

## Capabilities

### New Capabilities

None.

### Modified Capabilities
- `execution-cases`: a second scenario with its own model and a support matrix (B1-EXEC-001).

## Impact

Paths: `crates/pp-cube-conformance` (a second model, fixtures for both
dialects, cases, goldens, the matrix), `README.md`, `docs/guide.md`, one spec
and this change folder. Callers: none.
