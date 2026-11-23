# Changelog

Versions follow semantic versioning. Before 1.0 a minor version may change the
gRPC contract or the Rust API; such changes are listed under "Breaking".

The kernel version is part of every fingerprint, so fingerprints differ between
versions even when the SQL does not.

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
