# Notes

## 2026-11-23: measurement, design and implementation

The owner asked to continue with R4 after #75 merged. Fourth unit of the
productization route. Implemented with Claude Code (Anthropic).

Finding: every compilation validated and digested the whole model. At 50 cubes
that was 95% of a compilation (p50 11.5 ms, of which validate 9.4 and digest
1.5). The planned per-thread planner cache was therefore not built; see
design.md.

Done:
- `PreparedModel` (B0-RESULT-004); `compile` unchanged in behaviour.
- The service prepares a version at publish; compilations use it.
- `compile-bench` with padded models; `docs/performance.md` holds method and
  figures (one laptop, release build; ratios, not service targets).
- `Dockerfile`, `.dockerignore`, `docs/guide.md`, an example client
  (`crates/pp-cube-service/examples/compile.rs`), README link.

Evidence:
- `check ci: PASSED — 15 steps, 0 skipped, 257s @ 2155ff8+dirty` on the working
  tree that became the implementation commit. No golden changed.
- Benchmark, by hand: at 4 / 50 / 200 cubes, prepared p50 0.26 / 0.64 / 1.89 ms
  against 0.68 / 11.5 / 48.7 ms validating each time; 4-thread throughput
  12289 / 5434 / 1909 per second. The tool also asserts prepared and direct
  results are equal for every case.
- Image, by hand on local Docker (linux/amd64): built in 3m30s, 48.9 MB. A
  container listening on 0.0.0.0 with TLS and a read token compiled a paginated
  Presto query through the example client, refused a call without a token,
  served `/metrics`, ran as `nonroot`, and exited 0 on `docker stop`.
- Remote checks are recorded on the pull request. Cross-model review: not run.

Open: what remains of a compilation grows about 10 µs per cube and is not
profiled. The image is not built by CI and its base images are tags, not
digests; the workspace version is still 0.0.0. Those, the release workflow and
an aarch64 build are the harness pull request that follows.
