# Verification

`scripts/check.sh` is the only definition of passing. Read the script for the exact steps of each mode.

| Tier | Command | Where it runs | Blocks |
| --- | --- | --- | --- |
| T0 | `scripts/check.sh ci` | Every pull request | Merge |
| T1 | `scripts/check.sh conformance`: comparison with Cube Core | Nightly, on demand | Milestone |
| T2 | `scripts/check.sh engines` (prepared Presto) or `scripts/check.sh engines-container` (disposable Docker Presto) | On demand | Milestone |
| T3 | Generated SQL on real clusters, run by a person | Outside this repository | Milestone acceptance |

T1 is not built yet and exits 2. T2 has three Presto smoke cases (grouped sum, text equality binding and Tokyo day range); see
[engines.md](engines.md) for fixture setup and limits. It exits 2 without PRESTO_URL,
not a passing or complete T2 result. The container mode requires a Docker daemon,
provisions its own fixture and needs no PRESTO_URL. Doris and Druid execution are not built.
`scripts/dev deliver` requires both local `ci` and local `engines-container`
to pass before cross-vendor review, on every delivery round. T0 remains Docker-free.
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
