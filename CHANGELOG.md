# Changelog

Versions follow semantic versioning. Before 1.0 a minor version may change the
gRPC contract or the Rust API; such changes are listed under "Breaking".

The kernel version is part of every fingerprint, so fingerprints differ between
versions even when the SQL does not.

## Unreleased

**Compiles**
- Measures of more than one cube in one query (B1 multi-fact): each fact is
  aggregated on its own and the results are combined over the shared
  dimensions. Dimension, segment and security filters restrict every fact;
  measure filters, order and paging apply to the combined rows.
- Sum, avg, min and max of a cube reached through a many_to_one join, over
  distinct keys.
- A measure beside one of a cube joined one_to_one, and a formula over measures
  of both, in one SELECT.

- Rolling windows and period to date: `rolling_window` on a measure, with
  `trailing`, `leading` and `offset`, or `type: to_date` and a granularity.
- Time shifts: `multi_stage` number measures with `time_shift`, and formulas
  over shifted and unshifted measures.
- Ungrouped queries: `ungrouped: true` returns rows, measures unaggregated.
- Approximate distinct counts: the measure type `count_distinct_approx`.
- Declared granularities: `granularities` on a time dimension, by interval
  with an origin or an offset, and queries that group by one.
- Views: `views` in a model, taking dimensions and measures of cubes along
  join paths; queries on them; listed by GetMeta.

**Changed**
- SQL may begin with `WITH`. The result is still one statement.
- A granularity name no time dimension declares is a query error at
  compilation; it was an error when the query was read.
- Rust API: `Measure` has three more public fields and `Dimension` one;
  `MeasureType` and `Granularity` have a variant more each, and `Granularity`
  is no longer `Copy`; `Query::unsupported_b0_features` is now
  `unsupported_features`; `Model` has the field `views` and `Reference` a
  variant more.

**Breaking**
- A filter on a count of a cube reached through a many_to_one join returns
  unsupported. It compiled in 0.1.0 to a comparison of joined rows that could
  contradict the selected count.

**Evidence**
- Seventy shared cases on Presto 0.293, Doris 4.1.3 and against Cube Core 1.8.28.

## 0.1.0

The first version. It compiles the B0 baseline of Cube's data model and query
format to SQL and parameters for Presto and Apache Doris, and never executes SQL.

**Compiles**
- Single-cube aggregation (count, count_distinct, sum, avg, min, max), filtered
  measures and number formulas.
- Dimensions through many_to_one and one_to_one joins, including joins whose
  condition reads a third cube; measures of a joined cube where the result is
  still one SELECT.
- Every baseline filter operator with and/or groups, segments, absolute time
  ranges, day to year granularities with time zones, order, limit and offset.
- Security filters as top-level conditions that a user's filters cannot widen.

**Service**
- gRPC: Compile, ValidateModel, GetMeta, and opt-in PublishModel, ListModels and
  RetireModel. Typed parameters, structured lineage, declared engine session
  properties and an identity on every response.
- Bearer tokens in two roles, TLS, the standard health protocol, JSON request
  logs, Prometheus metrics, request bounds and graceful drain.

**Evidence**
- Forty-odd shared cases executed on Presto 0.293 and Doris 4.1.3, and compared
  with Cube Core 1.8.28, on every engine-relevant pull request. Cube Core's SQL
  and parameters equal the kernel's but for the default row limit Cube's API adds.

**Not included**
- Apache Druid; measures of more than one cube; anything that needs multi-stage
  SQL (multi-fact queries, time shifts, rolling windows, sums through a
  many_to_one join); views, ungrouped queries, approximate distinct counts,
  custom granularities and relative date ranges.
- Published images or packages. The image is built and verified in CI only.

See [the guide](docs/guide.md) and [performance](docs/performance.md).
