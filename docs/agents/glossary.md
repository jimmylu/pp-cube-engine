# Glossary

| Term | Meaning here |
| --- | --- |
| Contract | The input and output formats the kernel accepts and returns: Cube's data model format, Cube's query format, and the compile result. Narrowed to a baseline, not extended. |
| Baseline | The set of Cube features the kernel supports. It has two levels. |
| B0 | Baseline level 0: single-cube aggregation, filtered measures, ratios, many-to-one joins, filters and time ranges. The SQL is one flat `SELECT`. |
| B1 | Baseline level 1: multi-fact queries, time shifts, period-to-date, views, detail queries. The SQL uses common table expressions. |
| Kernel | The whole Rust workspace: a pure function from model, query and context to SQL and parameters. No IO, no credentials, no session state. |
| Upstream | Cube's repository at the pinned commit, in `vendor/cube`. |
| Planner | Upstream's `cubesqlplanner` crate: semantic planning and SQL generation. Not thread-safe. |
| Adapter | `pp-cube-adapter`: implements the bridge interfaces the planner requires. The only crate that touches the planner's interfaces. |
| Dialect | The templates and functions that make the planner emit SQL for one engine. Data files in `templates/`, code in `pp-cube-dialect`. |
| Facade | `pp-cube-compile`: the one entry point, `compile(model, query, context)`. |
| Capability | One area of specified behaviour, with its own spec under `openspec/specs/`. |
| Scenario | One testable WHEN/THEN case in a spec, with a stable id such as `B0-AGG-003`. |
| Golden | A stored expected SQL output, kept as a snapshot and compared on every test run. |
| Conformance | Comparing the kernel's output with Cube Core's for the same model and query. |
| Change | One unit of work: proposal, spec delta, design, tasks, then implementation. |
| Harness | Everything that controls how work is done and checked: CI, `scripts/`, agent instructions, lint and toolchain configuration. |
| Gate | A point where the owner's approval is required; mechanically, the `owner-approval` check. |
