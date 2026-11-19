# pp-cube-engine

A Rust compile kernel for semantic-layer queries. It takes a data model and a query written in the [Cube](https://cube.dev) model and query formats, plus a compile context, and returns SQL and parameters for one of three engines: Presto, Apache Doris or Apache Druid.

The kernel compiles; it does not execute. Running the SQL, access decisions and result handling belong to the caller.

## Status

Bootstrap. The repository holds the engineering environment only; there is no product code yet.

## Approach

- The contract is Cube's data model and query format, narrowed to a baseline and not extended.
- Planning is done by Cube's Rust planner, `cubesqlplanner`, used as it is at a pinned upstream commit. This repository adds an adapter layer, dialect templates for the three engines, a facade and a service.
- The baseline has two levels. B0 covers single-cube aggregation, filtered measures, ratios, many-to-one joins, filters and time ranges. B1 covers multi-fact queries and time-shifted measures.

## How changes are made

Most of the code is written by coding agents, under these controls:

- Every change starts as a spec proposal and is implemented only after the proposal is approved.
- `main` accepts changes only through pull requests. A pull request merges only after the `t0` check passes and the repository owner approves the `owner-approval` check.
- The model that reviews a pull request comes from a different vendor than the model that wrote it.

## Contributing

The project is in early personal development and is not accepting outside contributions yet. Issues are welcome.

## Licence

Apache-2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE).
