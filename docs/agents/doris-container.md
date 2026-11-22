# Doris local execution

The owner requires real local Docker Doris execution before delivery of Doris
changes. This isolated harness PR adds a pinned Apache Doris 4.1.3 amd64 FE+BE
container. No product SQL, snapshots, dependencies or vendor source change.

Run `scripts/check.sh doris-container`. Python 3 with venv/pip and a Docker daemon
with amd64 support are required. The tier installs mysql-connector-python 9.4.0
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

This is stored-golden execution, not live compilation or full B0/T2 acceptance.
It does not validate time dimensions or all filter/NULL/type combinations.

The delivery controller runs local CI, Presto Docker and Doris Docker on every
round before cross-vendor review. T0 remains Docker-free. Engine or Docker
failure leaves the PR Draft. Cleanup failure also fails the run. SIGKILL can
leave a uniquely named cube-doris- container requiring manual removal.

Sources:
- Official image source: https://github.com/apache/doris/tree/master/docker
- Prepared cursor protocol: https://dev.mysql.com/doc/connector-python/en/connector-python-api-mysqlcursorprepared.html
- Docker image: https://hub.docker.com/r/apache/doris

Handoff: real local Docker execution passed during development; final delivery reruns it on the published head. Final evidence
belongs in SHA-bound PR comments. The owner performs final review and merge.
