# Verification

`scripts/check.sh` is the only definition of passing. Read the script for the exact steps of each mode.

| Tier | Command | Where it runs | Blocks |
| --- | --- | --- | --- |
| T0 | `scripts/check.sh ci` | Every pull request | Merge |
| T1 | `scripts/check.sh conformance`: the kernel's Presto SQL against pinned Cube Core | GitHub `engines` workflow on pull requests that can affect engines; locally on demand | Milestone |
| T2 | `scripts/check.sh engines` (prepared Presto), `scripts/check.sh engines-container` (disposable Docker Presto) or `scripts/check.sh doris-container` (disposable Docker Doris) | GitHub `engines` workflow on pull requests that can affect engines; locally on demand | Milestone |
| T3 | Generated SQL on real clusters, run by a person | Outside this repository | Milestone acceptance |

T1 compares the kernel's Presto SQL and its results with Cube Core at the release of the
upstream pin; see [cube-core.md](cube-core.md). The Presto and Doris tiers execute the live-compiled
execution manifest of `pp-cube-conformance`, the same cases on both engines; see
[engines.md](engines.md) and [doris-container.md](doris-container.md). `engines` exits 2
without PRESTO_URL, which is not a passing result. The container modes need a Docker
daemon and the pinned Rust toolchain and provision their own fixtures. Druid execution
is not built.
## Build checks

`scripts/check.sh image` builds the service image and verifies a container made from
it: an encrypted, authenticated compilation through the example client, refusal of a
caller without a token, the metrics endpoint, a non-root user and a clean stop. It
needs Docker, openssl, curl and the Rust toolchain, publishes nothing and removes
the image afterwards. The `build` workflow runs it as `image`, and runs the whole
test suite on an aarch64 runner as `aarch64`. Both are slow, so on pull requests
they run only when a file that decides the build changes (the Dockerfile and its
ignore file, the Cargo manifests and lockfile, the toolchain, the image script,
`scripts/vendor-sync.sh`, the workflow); they run on every push to main, which is
where a change to the crates alone is covered. A failure on main is fixed forward.

`scripts/dev deliver` requires local `ci`, remote `t0`, remote `image` and `aarch64`
when the `build` workflow is triggered and, when the `engines` workflow is
triggered, remote `presto-smoke`, `doris-smoke` and `cube-core`. It starts no local container and runs
no review; see [delivery.md](delivery.md). T0 remains Docker-free.
Results from T3 are recorded publicly only as pass or fail per scenario.

## Tools

The Rust toolchain is pinned by `rust-toolchain.toml` and installs itself through `rustup`. `ci` mode also needs `cargo-deny`, `typos`, `taplo`, Python 3 for controller regression tests and, once `openspec/` exists, the `openspec` CLI at the version named in `scripts/check.sh`. `actionlint` and `gitleaks` run when installed and are reported as skipped when not. Versions used in CI are in `.github/workflows/t0.yml`.

## Reading a failure

The script stops at the first failing step and names it: `check ci: FAILED at "<step>"`.

| Step | Usual cause |
| --- | --- |
| `vendor` | `scripts/vendor-sync.sh` not run, or a file under `vendor/` was edited |
| `upstream` | The toolchain channel or `[patch]` section no longer matches the pinned upstream commit |
| `lockfile` | A dependency changed without `Cargo.lock`; a `Cargo.lock` change is labelled `risk:upstream` |
| `tests` with pending snapshots | A golden output changed; see hard rule 3 in `AGENTS.md` |
| `deny` | A new dependency brings a licence, source or advisory that `deny.toml` does not allow; changing `deny.toml` is a harness change |
| `traceability` | A scenario in `openspec/specs/` without a test, or a test naming an id that is in neither `openspec/specs/` nor the spec delta of an active change |
| `harness-only` | Harness files and other files in the same change; split it |

When the same step fails twice after two different fixes, that is a stop condition.

## Snapshot tests

Golden SQL is stored as `insta` snapshots. The check runs with `INSTA_UPDATE=no`, so a changed output fails instead of being rewritten.

## Scenario ids

A scenario title in a spec starts with its id: `#### Scenario: B0-AGG-003 Sum over one cube`. A test covers it with `// scenario: B0-AGG-003`. `scripts/traceability.sh` compares the two sets.

## Dependency license decision #50

The owner approved adding Zlib to the explicit SPDX license allowlist for the
gRPC code-generation dependency chain: tonic-prost-build -> prost-build ->
petgraph -> hashbrown -> foldhash 0.1.5. This permits the declared license,
not an unreviewed dependency or an advisory exception. Attribution and license
notices remain the distributor's responsibility. Other licenses still need an
explicit policy decision; do not automatically allow every new dependency.

This isolated harness change edits only the license policy and this record.
Product PR #51 keeps its own dependencies, implementation and acceptance gates.
After this prerequisite merges, resume its checks and cross-vendor review on
the final product head. SHA-bound validation evidence belongs in PR comments.
