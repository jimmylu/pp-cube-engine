# Cube Core comparison (T1)

`scripts/check.sh conformance` compares the kernel's Presto SQL with Cube Core's
own, case by case. It is the only check that asks Cube itself; goldens show that
output is stable, this shows that it agrees with Cube.

## What it does

1. Checks that the pinned Cube Core image is the release of the upstream pin
   (the gitlink of `vendor/cube` equals the commit of the release tag).
2. Compiles the Presto execution manifest with sources:
   `cargo run -q --locked -p pp-cube-conformance --bin execution-cases -- presto --sources`.
   A compiler failure starts no container.
3. Starts one disposable Cube Core container in development mode with the
   manifest's model text as its model file. Cube is told each data source is
   `prestodb` so it has a dialect; it never connects to a database.
4. Starts the disposable Presto of the Presto tier and, for every case:
   - asks Cube Core `GET /cubejs-api/v1/sql` for the case's query, with its
     effective timezone stated and its security filters appended as top-level
     filters, which is where the kernel puts them;
   - requires Cube's SQL and parameters to equal the kernel's. One difference is
     allowed and expected: Cube's API adds `LIMIT 10000` to a query that sets no
     limit, and the kernel adds none;
   - executes Cube's SQL on Presto with the manifest's session properties and
     requires the case's expected columns and rows.
5. Removes both containers.

A pass is tier T1 for the cases, scenario B0-PRESTO-002. Cube has no Doris
dialect, so there is no Doris comparison. The Druid comparison runs inside the
Druid tier, where a case may state that Cube Core refuses it or writes SQL Druid rejects; see [druid-container.md](druid-container.md).

## Run it

Delivery does not start it locally. The `cube-core` job of the `engines`
workflow runs it on GitHub for every pull request that can affect it, and
`scripts/dev deliver` requires it when triggered. Locally it needs Docker with
amd64 support and the pinned Rust toolchain; the Cube image is about 1 GB.

A failure line names a fixed category, the stage and the case, never SQL, an
endpoint or a server message. `Cube Core SQL differs from the kernel` at
`stage=cube-core-sql` is the interesting one: reproduce it by printing both
statements for that case from the sources manifest and Cube's answer.

## Moving the upstream pin

`CUBE_VERSION`, `CUBE_COMMIT` and `CUBE_IMAGE` in `scripts/engines/cube-core.py`
name one Cube release. When the pin moves, the first step fails until they name
the release tagged at the new commit: find the tag whose commit is the new
gitlink, and take the digest of `cubejs/cube:v<version>` for linux/amd64. A pin
that is not a release tag has no image to compare with; report that instead of
comparing against a neighbouring release.

## What it does not show

Forty cases on one small model are agreement on B0, not a proof of
equivalence. Which of Cube's planners answered (its Rust planner or its
JavaScript one) was not determined; it is whatever that release uses by default.
