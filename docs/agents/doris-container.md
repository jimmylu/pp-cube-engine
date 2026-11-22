# Doris local execution

The owner requires real local Docker Doris execution before delivery of Doris
changes. This isolated harness PR adds a pinned Apache Doris 4.1.3 amd64 FE+BE
container. No product SQL, snapshots, dependencies or vendor source change.

Run `scripts/check.sh doris-container`. Python 3 with venv/pip, the pinned Rust
toolchain and a Docker daemon with amd64 support are required. The tier installs mysql-connector-python 9.4.0
into the worktree Git directory, outside tracked files. It needs network access
for the connector and image. The disposable engine has a 6 GiB memory limit and
an ephemeral loopback-only MySQL port. No pre-existing containers are removed.

The suite reads checked-in Doris compiler goldens. It executes aggregate
pagination, a two-hop dimension join, six literal pattern operators and mandatory
security conjoined with a user OR. Parameters use the native MySQL prepared
statement protocol, never SQL text substitution. Case-specific fixture parameters
are used for the six pattern templates to avoid an empty result caused by their
original contradictory equality filter. One OFFSET variant checks nonempty
aggregate values as well as the original empty pagination result. Column names
and ordered rows are asserted against independent literal expectations.

## Live-compiled manifest

The tier then runs the execution manifest of `pp-cube-conformance` in the same
container. Before any container is created it runs
`cargo run -q --locked -p pp-cube-conformance --bin execution-cases -- doris`,
so the SQL is compiled from the checked-out sources and a compiler failure never
starts an engine. The kernel stays compile-only: only this tier executes SQL.

Each case carries its fixture statements, SQL, typed ordered parameters and
literal expected columns and rows. The tier drops the stored-golden `dw` schema,
loads a fixture once for consecutive cases that share it, binds parameters
through the native prepared protocol and compares columns and ordered rows.
An unknown manifest schema, dialect, case field, parameter type or cell shape
fails the run. Cases and expectations are product files; add them there, not here.

The manifest currently holds thirteen temporal cases for B0-DORIS-010: five
calendar granularities, six date operators, Tokyo millisecond bounds, a New York
DST day and a security date range conjoined with a user OR. The final line
reports both counts. This is still not full B0/T2 acceptance: the stored-golden
part does not compile live, and not every filter/NULL/type combination runs.

The delivery controller runs local CI, Presto Docker and Doris Docker on every
round before cross-vendor review. T0 remains Docker-free. Engine or Docker
failure leaves the PR Draft. Cleanup failure also fails the run. SIGKILL can
leave a uniquely named cube-doris- container requiring manual removal.

Sources:
- Official image source: https://github.com/apache/doris/tree/master/docker
- Prepared cursor protocol: https://dev.mysql.com/doc/connector-python/en/connector-python-api-mysqlcursorprepared.html
- Docker image: https://hub.docker.com/r/apache/doris

Handoff: issue #59 is resolved by the manifest integration above; real local
Docker execution passed during development and delivery reruns it on the
published head. Still open, as product work: transfer B0-DORIS-010 out of
`seed-baseline-specs` and delete the transitional opt-in test and verifier in
`crates/pp-cube-compile/tests`; move the stored-golden assertions into manifest
cases. Final evidence belongs in SHA-bound PR comments. The owner performs final
review and merge.
