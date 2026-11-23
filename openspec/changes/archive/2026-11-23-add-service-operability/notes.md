# Notes

## 2026-11-23: design and implementation

The owner asked to continue with the recommended roadmap after #74 merged.
Third unit of the productization route. Implemented with Claude Code (Anthropic).

Done:
- gRPC layer: `Access` (read and admin bearer tokens), `Limits` (compilation
  deadline), `Telemetry` (counts, duration histogram, JSON log events),
  `HealthService` (standard `grpc.health.v1`).
- Core: `with_max_versions`; publishing beyond it is refused.
- Executable: TLS certificate and key, token files, the rule that a non-loopback
  listen needs TLS and a read token, `--metrics-listen`, limit options, JSON
  logs on stderr, SIGTERM/SIGINT drain with a drain time.
- Service README rewritten as an operator's guide.

Dependencies: tonic gains `tls-ring`. `Cargo.lock` gained two dependency edges
and no package; rustls and ring were already resolved, so `deny.toml` and the
licence allowlist are untouched.

Evidence: `check ci: PASSED — 15 steps, 0 skipped, 184s @ 899a8e0+dirty` on the
working tree that became the implementation commit. Earlier runs of the same
change failed in turn on one clippy lint, on the deadline test (its recovery
assertion allowed a single 50 ms deadline and lost under parallel load; it now
retries) and on manifest formatting; each was fixed. No golden changed. Remote
checks are recorded on the pull request. Cross-model review: not run.

Not covered by a test: the exit path when requests outlast the drain time, and
signal handling on non-Unix platforms.

Open (route): performance and release next. Known limits stated in design.md:
static tokens rotate by restart and name roles, not callers; no mTLS; a running
compilation cannot be cancelled; published versions are still in memory only.

Learned: rustls rejects a CA certificate presented by a server
(`CaUsedAsEndEntity`), so `openssl req -x509` needs `basicConstraints=CA:FALSE`.
tonic applies TLS to a caller-supplied incoming stream, so the bound listener
can still be announced before serving.
