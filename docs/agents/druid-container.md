# Druid execution and Cube Core comparison

`scripts/check.sh druid-container` runs pinned Apache Druid 26.0.0 (the version
in production) in one disposable container, executes the live-compiled Druid
execution manifest of `pp-cube-conformance` on it and, in the same run, compares
every case's SQL with pinned Cube Core's Druid dialect. Druid takes minutes to
start and to load, so both checks share one engine. It needs Docker with amd64
support and the pinned Rust toolchain, and no Python package.

## What runs

1. `cargo run -q --locked -p pp-cube-conformance --bin execution-cases -- druid --sources`
   compiles the manifest before any container exists. It holds the cases the
   Druid dialect compiles; the cases it refuses are a reviewed golden of T0, not
   part of this tier.
2. One container runs ZooKeeper and the five Druid services by the image's own
   scripts, from its `nano-quickstart` configuration with two additions, both
   in `scripts/engines/druid-container.py`:
   - `druid.generic.useDefaultValueForNull=false`. By default Druid 26 stores a
     NULL number as 0 and a NULL string as empty; the expectations are those of
     the other engines. In default mode one case differs.
   - `druid.processing.numMergeBuffers=6` on broker and historical. Two exact
     distinct counts in one query need three merge buffers; the example has two.
3. Only the router port is published, on loopback, at an ephemeral port. The
   tier waits until SQL is answered and `/status` reports exactly 26.0.0.
4. Each fixture statement is a SQL ingestion (`REPLACE INTO ... OVERWRITE ALL
   ... EXTERN(inline)`), submitted to `/druid/v2/sql/task`. It counts as loaded
   when the task succeeded, a segment version that did not exist before is
   served and no older one is. Do not wait on a row count: it passes on the
   data of the fixture before.
5. Every case: Cube Core is asked `GET /cubejs-api/v1/sql` as in
   [cube-core.md](cube-core.md), and its SQL and parameters must equal the
   kernel's but for Cube's default `LIMIT 10000`. Then the kernel's SQL runs
   through `/druid/v2/sql`, unchanged, with its parameters apart and the
   manifest's session as query context (`useApproximateCountDistinct=false`),
   and must return the case's columns and rows.
6. Both containers are removed, and only those the run created.

A pass is B0-DRUID-004 (execution) and B0-DRUID-003 (Cube Core comparison) for
the cases. `python3 scripts/engines/druid-container.py --no-cube-core` skips the
comparison for local debugging; its final line says so, and it is not the tier.

A failure line names a fixed category, the stage (`provision`, `cube-core`,
`readiness-version`, `fixture`, `cube-core-sql`, `execution`), the case and,
for a fixture, the statement number; never SQL, an endpoint or a server
message. To see Druid's own message, rerun locally and query the engine by hand
before the container is removed.

## Delivery

Delivery does not start this tier locally. The `druid-smoke` job of the
`engines` workflow runs it on GitHub for every pull request that can affect it,
and `scripts/dev deliver` requires it when triggered. A run takes about ten
minutes, most of it ingestion: each fixture loads up to nine datasources and
consecutive cases share one.

## What it does not show

The container uses the image's smallest configuration with the two settings
above. Whether production runs SQL-compatible NULL handling, and with how many
merge buffers, is the owner's to confirm; a cluster in default NULL mode
returns different rows for a filter such as `IS NOT NULL` on a number. Real
clusters are tier T3.
