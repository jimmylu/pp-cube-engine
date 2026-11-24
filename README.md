# pp-cube-engine

A Rust compile kernel for semantic-layer queries. It takes a data model and a query written in the [Cube](https://cube.dev) model and query formats, plus a compile context, and returns SQL and parameters for one of three engines: Presto, Apache Doris or Apache Druid.

The kernel compiles; it does not execute. Running the SQL, access decisions and result handling belong to the caller.

## Status

In development; not yet released. The B0 baseline and, of B1, multi-fact
queries, rolling windows, period to date, time shifts, ungrouped rows,
approximate distinct counts, declared granularities and views compile for Presto
and Apache Doris, and eighty-five shared cases, a property-insurance scenario
among them, are executed on both engines for every relevant pull request. A gRPC service exposes Compile, ValidateModel and GetMeta
([crates/pp-cube-service](crates/pp-cube-service/README.md)). Apache Druid
compiles for the B0 baseline and is not yet executed in the checks; production
packaging is not built.
Implemented behaviour is recorded in [openspec/specs](openspec/specs).

Start with [the guide](docs/guide.md); latency figures are in [performance](docs/performance.md).

## Approach

- The contract is Cube's data model and query format, narrowed to a baseline and not extended.
- Planning is done by Cube's Rust planner, `cubesqlplanner`, used as it is at a pinned upstream commit. This repository adds an adapter layer, dialect templates for the three engines, a facade and a service.
- The baseline has two levels. B0 covers single-cube aggregation, filtered measures, ratios, many-to-one joins, filters and time ranges. B1 covers multi-fact queries and time-shifted measures.

## How changes are made

Most of the code is written by coding agents, under these controls:

- Every change carries a spec proposal, its implementation and its tests in one pull request, sized as one capability.
- `main` accepts changes only through pull requests. A pull request merges only after the `t0` check passes and the repository owner approves the `owner-approval` check.
- Review by a second model, from a different vendor than the one that wrote the change, is run when the owner asks for it.

## Contributing

The project is in early personal development and is not accepting outside contributions yet. Issues are welcome.

## Licence

Apache-2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE).
