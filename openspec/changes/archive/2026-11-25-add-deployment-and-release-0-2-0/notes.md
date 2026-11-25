# Notes

## Done

- `deploy/kubernetes.yaml` and `docs/deployment.md`; the workspace is 0.2.0 and the changelog has its section.
- B1-API-004: the service starts with the manifest's own arguments, answers the probe with the store's model version and exits cleanly on SIGTERM.
- `scripts/release.sh check v0.2.0` passes on this tree (the check that the commit is on main applies once it is merged).

## Evidence

- `cargo test --workspace` passes at 0.2.0: no test or golden depended on the version's value.
- The remote checks on the delivered head are the evidence of record, `image` and `aarch64` among them since `Cargo.toml` changed; see the pull request.

## Learned

- A Kubernetes gRPC probe cannot speak TLS, and the image has no shell; the metrics endpoint is the one thing a probe can ask, and it happens to mean ready.
- macOS keeps its temporary files under `/var/folders`, so a test that looks for "a path under /var" finds its own scratch files. Compare with the manifest's own prefixes.

## Open

- The `publish` job of the release workflow has never run. The first tag is its first run; `docs/agents/release.md` says what to do if it fails part way.
- Resource requests and limits in the manifest are not measured (G10).
- The platform is assumed to be Kubernetes.
- The package's visibility on GitHub is the owner's to set after the first image.
