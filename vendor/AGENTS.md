# vendor/

`vendor/cube` is upstream Cube (Apache-2.0) as a git submodule, pinned to one commit and checked out sparsely by `scripts/vendor-sync.sh`. It is read-only: `scripts/check.sh` fails when any file under it differs from the pinned commit.

- The planner is `vendor/cube/rust/cube/cubesqlplanner/cubesqlplanner`. Its test fixtures (`src/test_fixtures`) are the reference for how the bridge interfaces are implemented.
- Only `rust/cube` and `rust/cubesql/cubeclient` are present. Other parts of Cube, including the JavaScript dialect templates, are read from the same commit on GitHub.
- The root `rust-toolchain.toml` channel and the root `[patch]` section mirror upstream's; the check compares them.
- Moving the pin is its own change, labelled `risk:upstream`: update the submodule commit, re-run `scripts/vendor-sync.sh`, re-align toolchain, `[patch]` and `Cargo.lock`, and review every golden SQL difference.
- Code copied from upstream into `crates/` keeps its Apache-2.0 notice and names the upstream path and commit it came from.
