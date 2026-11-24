# Design

## Context

See proposal.md and review.md. No real model or query of the domain exists
yet, so one was invented: neutral names, nine policy terms, and the questions
an underwriting, a claims and a reinsurance analyst would ask first.

## The model

`crates/pp-cube-conformance/fixtures/insurance.yaml`, inside the contract.

| Cube | Grain | Notable members |
| --- | --- | --- |
| `policies` | One policy term; a renewal names the term it renews | written premium, sum insured, renewal share (filtered count over count), rate per mille, year and quarter to date, prior year and growth, an underwriting year that starts in April, an approximate insured count |
| `earned_premiums` | Policy and month | earned premium, exposure, rolling twelve months |
| `claims` | One claim | counts, open count, report lag, large-loss segment; joined one_to_one to its reserve |
| `claim_reserves` | At most one row per claim | outstanding |
| `claim_payments` | One payment | paid, indemnity and expense split, paid per claim, year to date |
| `cessions` | Policy and treaty | ceded premium |
| `products`, `channels`, `branches` | Dimensions | line of business, channel, branch and region |

Two views, `underwriting` and `claims_overview`, present them as an analyst
would see them. Facts meet only through `policies`.

## The support matrix

Thirty-three queries, compiled for Presto and Doris and put to Cube Core
1.8.28. The reviewed outcome per dialect is the golden
`scenario__insurance_support_matrix`.

| Outcome | Queries | |
| --- | --- | --- |
| Compiled, SQL byte-identical to Cube Core's | 24 | Premium by line and channel, renewal share, rates, year and quarter to date by month, growth on the prior year, underwriting year, exact and approximate insured counts, top branches, policy and large-loss listings, earned premium and exposure, rolling twelve months, written against earned, claim counts and lags, paid split and severity, the development triangle, three and five facts side by side, both views |
| Refused | 9 | Below |

| Refused query | Here | Cube Core | Needed by the domain | Follow-up |
| --- | --- | --- | --- | --- |
| Outstanding reserves by line; paid and outstanding by line | query error | compiles | Yes | #88, a defect: found here, not known before |
| Loss ratio as a measure over two facts | query error | error | Yes, first of all | #89 |
| Earned premium and paid claims by month | query error | error | Yes | #89 |
| Year to date by quarter | unsupported on Presto, compiles on Doris | compiles to SQL Presto rejects | Yes | #90 |
| Lines with more than one claim (filter on a count over repeated rows) | unsupported | compiles, comparing joined rows | Sometimes | #83 |
| Last twelve months as a relative range | not read | compiles | Yes, but the caller can resolve dates | Listed in the guide; by design the kernel reads no clock |
| Two years compared with `compareDateRange` | unsupported | two statements | The prior-year measure covers it | Listed in the guide |
| Claims reported by hour | unsupported | compiles | Rarely | None |

What the scenario shows beyond the list:

- **The model had to be shaped around one rule.** A branch reached both from
  the policy and from an intermediary would be two join paths, which the
  kernel refuses for every query that touches both. The scenario has no such
  diamond; a real model is likely to. Cube picks a path silently.
- **Count ratios need a cast.** `renewed / policies` is integer division on
  Presto. The model writes `CAST(... AS DOUBLE)`, which both dialects take; a
  model author has to know.
- **Long SQL meets a transport limit on Presto.** See the decision on the
  growth query below: a caller that binds parameters through Presto's
  protocol or JDBC driver hits the server's 8 KB header limit with ordinary
  questions. It is a deployment setting, and it belongs in the go-live list.
- **Everything the route built is used.** Multi-fact queries, rolling windows,
  time shifts, declared granularities, ungrouped rows, approximate counts and
  views all appear in ordinary questions of the domain.

## Decisions

- **One model document for both scenarios.** The manifest carries one model,
  which the Cube Core comparison loads as one file. The insurance cubes and
  views are appended to the retail ones by text, so each file stays readable
  on its own and no harness path changes. Existing cases compile to the same
  SQL: the manifest goldens gain blocks and lose no line.
- **Thirteen execution cases, not thirty-three.** Chosen to cover each kind of
  question once with results that are exact on both engines: cases whose
  answer is a repeating decimal (an average of 4900 over 3) are surveyed and
  compared with Cube Core's text by hand, not executed.
- **Growth on the prior year is surveyed, not executed.** Its Presto SQL is
  8.4 KB once encoded. The execution tier sends a parameterized statement in
  the `X-Presto-Prepared-Statement` header, as Presto's protocol and its JDBC
  driver do, and Presto's default header limit is 8 KB, so the request was
  refused with an HTTP error before it ran. Run by hand with its values
  written in, on Presto 0.293, it returns the expected rows; on Doris it ran
  in the tier and passed. The tier is a harness path and is not changed here.
- **Expected rows are computed from the fixture rows by a separate script in
  plain Python**, with no SQL, and then written into the cases.
- **Nothing the survey found is fixed here.** #88 is a defect in the join
  check of multi-fact queries, outside this change's paths; it gets its own
  change, and until then the matrix records the refusal.

## Risks / Trade-offs

- An invented scenario is not the real one → it replaces a guess with a
  reviewed list, and is to be replaced in turn when real models exist.
- "Needed by the domain" is this session's judgement, not the owner's → the
  column is for the owner to correct.

## Migration Plan

Additive: tests, fixtures and documents. Evidence: `scripts/check.sh ci`,
remote `t0`, `presto-smoke`, `doris-smoke`, `cube-core`.
