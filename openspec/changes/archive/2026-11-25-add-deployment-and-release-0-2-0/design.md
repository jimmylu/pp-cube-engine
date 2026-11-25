# Design

## Decisions

1. **Kubernetes, stated as an assumption.** The owner asked for three replicas that scale out easily and named no platform. Kubernetes is the common case and the manifest is plain, without Helm or Kustomize: one file to read and edit. `docs/deployment.md` says what another supervisor has to do instead.
2. **Probes ask the metrics port.** The service requires TLS beyond loopback, and a Kubernetes gRPC probe cannot speak TLS. The image has no shell or client for an exec probe. `GET /metrics` answers only after the models are validated and both ports are bound, which is readiness. The cost: the metrics port listens on the pod address, without authentication, so the Service does not carry it.
3. **The test starts the real binary with the manifest's arguments.** It reads the arguments from the file, replaces only the mounted paths by files it makes and the ports by free ones, and waits for `listening`, a `200` from `/metrics` showing one model version, and a clean exit on SIGTERM. A manifest with an option the service lacks, or without one it requires, fails the test. It also checks the three numbers the availability claim rests on (3 replicas, 1 unavailable, at least 2) and that the grace period exceeds the drain. The arguments are written one quoted string a line so the test needs no YAML parser; the service crate has none.
4. **Resources are starting values.** Two cores follow `--workers 2`; the memory figures are not measured. The manifest and the guide say so. Measuring is route item G10.
5. **The image is named by its public address.** `ghcr.io/jimmylu/pp-cube-engine` is the registry of this public repository and already follows from `repository` in `Cargo.toml`. The manifest names the version; the guide tells operators to deploy the digest they verified.
6. **0.2.0.** The owner agreed the number on 2026-11-25. A minor version before 1.0 may break; the changelog lists what does. The tag is the owner's to push.

## Not in this change

- Measured sizing and a load test (G10).
- A canary procedure of our own; the guide points at the platform's.
- Clients for Java, Python and Node.js (G6).
