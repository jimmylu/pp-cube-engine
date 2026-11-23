# Doris local execution

The owner requires real local Docker Doris execution before delivery. The tier
runs a pinned Apache Doris 4.1.3 amd64 FE+BE container and executes the
live-compiled execution manifest of `pp-cube-conformance`. It reads no stored
snapshot and holds no fixture or expectation of its own.

Run `scripts/check.sh doris-container`. Python 3 with venv/pip, the pinned Rust
toolchain and a Docker daemon with amd64 support are required. The tier installs
mysql-connector-python 9.4.0 into the worktree Git directory, outside tracked
files. It needs network access for the connector and image. The disposable
engine has a 6 GiB memory limit and an ephemeral loopback-only MySQL port. No
pre-existing containers are removed.

## What runs

Before any container is created the tier runs
`cargo run -q --locked -p pp-cube-conformance --bin execution-cases -- doris`,
so the SQL is compiled from the checked-out sources and a compiler failure never
starts an engine. The kernel stays compile-only: only this tier executes SQL.

Each case carries its fixture statements, SQL, typed ordered parameters and
literal expected columns and rows. The tier loads a fixture once for consecutive
cases that share it, binds parameters through the native MySQL prepared
statement protocol, never SQL text substitution, and compares column names and
ordered rows. An unknown manifest schema, dialect, case field, parameter type or
cell shape fails the run. A failure line names the stage, the case, the
numeric server code and fixed phrases found in the message, never the server message,
SQL or endpoint.

Readiness has two steps. `SHOW BACKENDS` must report every backend alive, and then a
one-replica probe table must really be created: a backend can be alive before its
storage is registered with the frontend, and the first `CREATE TABLE` then fails
with code 1105 (seen on GitHub runners at `stage=fixture case=day statement=2`).
Only that condition is retried, for up to two minutes; the probe database is dropped.

Cases, fixtures, expectations and their reviewed golden are product files in
`crates/pp-cube-conformance`; add or change them there, in a product change.
The final line reports the number of cases and the scenario ids they carry.
Those scenarios (B0-DORIS-002 for the B0 matrix, B0-DORIS-010 for temporal
cases) are specified in `openspec/specs/dialect-doris`. A pass here is Doris T2
for those cases; it says nothing about Presto, Druid, Cube Core comparison (T1)
or real clusters (T3).

## Delivery

Delivery does not start this tier locally. The `doris-smoke` job of the `engines`
workflow runs it on GitHub for every pull request that can affect it, after
raising `vm.max_map_count` and disabling swap on the runner, and
`scripts/dev deliver` requires it when triggered. Run the tier locally when
developing cases or debugging a failure. T0 remains Docker-free. Cleanup failure
fails the run. SIGKILL can leave a uniquely named cube-doris- container requiring
manual removal.

Sources:
- Official image source: https://github.com/apache/doris/tree/master/docker
- Prepared cursor protocol: https://dev.mysql.com/doc/connector-python/en/connector-python-api-mysqlcursorprepared.html
- Docker image: https://hub.docker.com/r/apache/doris

## History and handoff

The tier first executed nineteen assertions against stored compiler goldens
with its own fixture (#57). The manifest was integrated beside them (#62,
closing #59), then grew to cover the whole B0 matrix (#64). The stored-golden
assertions and `scripts/engines/doris-fixture.sql` are removed. Each of them has
a manifest case: aggregates, a page inside and a page past the last group,
measure filter, joined dimension, six pattern operators with a wildcard decoy
and NULL-keeping negations, and a security pattern beside a user OR. Open: the Presto tier still reads stored goldens (#61).
Final evidence belongs in SHA-bound PR comments. The owner performs final
review and merge.
