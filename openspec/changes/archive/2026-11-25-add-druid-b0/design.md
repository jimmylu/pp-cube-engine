# Design

## Established on Druid 26.0.0 before designing

One container of the published `apache/druid:26.0.0` image running ZooKeeper
and the five services with the image's own scripts (about 2.6 GiB); rows
loaded by SQL ingestion of inline data.

| Question | Finding |
| --- | --- |
| Does Cube's Druid SQL equal the kernel's? | Cube Core 1.8.28 with its Druid dialect was asked for all 85 shared cases. For the 69 the kernel compiled without refusals, SQL and parameters are byte-identical (but for Cube's default row limit) |
| Do joins run? | Yes: every joined-dimension case passes, two hops and security-only joins included |
| Does multi-fact SQL run? | No. A join on `a = b OR (a IS NULL AND b IS NULL)` is "not supported", as is the union inside a subquery. Only the form without dimensions (a cross join of totals) ran |
| Ungrouped rows? | "SQL query requires order by non-time column, which is not supported" |
| Schema and time | Datasources live in `druid`; `dw.x` is "Object 'dw' not found". A datasource has one timestamp column, `__time` |
| Parameters | Bound through the SQL API's `parameters`, typed, in order |
| Exact distinct counts | Need the context `useApproximateCountDistinct=false`; two in one query need three merge buffers, and the smallest example configuration has two |
| NULL handling | Druid 26's default stores a NULL number as 0. One case of the 52 then differs (a count filtered on `IS NOT NULL`). With `druid.generic.useDefaultValueForNull=false` all 52 pass |

## Decisions

- **Refusals are data.** `refuses: [...]` in a dialect's file, from a fixed
  list the dialect crate validates. The adapter checks four of them from the
  query and the model before planning (ungrouped, rolling window, time shift,
  approximate distinct, declared granularity) and one from the SQL after
  planning (more than one SELECT), because only the planner knows when it
  leaves one SELECT. Alternative: infer from missing templates; rejected
  because a missing template is an internal error, which a caller cannot act
  on, and five cases did fail that way before this.
- **Multi-stage is refused as a whole.** Totals of two facts without a
  dimension do run on Druid; they are refused with the rest. Telling the
  shapes apart belongs to G4.
- **A session property every statement declares.** The dialect type gains
  settings that apply always, beside the ones a page needs. Druid's is the
  exact-distinct context. A caller that ignores it gets estimates.
- **The model is the deployment's.** Tables are `druid.<datasource>` and time
  is `__time`; the conformance crate rewrites the shared model that way for
  Druid, as a Druid deployment would write its own. The kernel does not
  rewrite anything.
- **Fixtures are SQL too.** One `REPLACE INTO … OVERWRITE ALL SELECT … FROM
  TABLE(EXTERN(inline))` a table that has rows, generated from the Presto
  fixtures so the engines hold the same rows. A tier submits each as an
  ingestion task and waits until the new segments, and no older ones, are
  served: waiting for a row count alone picked up the previous fixture.
- **A manifest of part of the cases.** The Druid manifest holds what compiles;
  the reviewed list `druid_support` gives every other case its refusal. Any
  other failure fails the manifest.

## Risks / Trade-offs

- No Druid tier in the checks yet → the SQL is pinned by goldens and equals
  Cube Core's; execution evidence is the manual run in notes.md until the
  harness change.
- Production's NULL mode is not known → the guide says what differs.
- `__time` only: a cube with two time columns has one on Druid.

## Migration Plan

Additive. Evidence: `scripts/check.sh ci`, remote `t0`, `presto-smoke`,
`doris-smoke`, `cube-core`.
