# Deploying and upgrading

The service is one stateless process that compiles and never executes SQL.
Replicas share nothing but the models they read, so it scales by adding
replicas. [deploy/kubernetes.yaml](../deploy/kubernetes.yaml) runs three of
them; this page explains it and what to do on an upgrade. The options
themselves are in the [service guide](../crates/pp-cube-service/README.md).

## A release

A release is the image `ghcr.io/jimmylu/pp-cube-engine:<version>`, for
linux/amd64 and linux/arm64, and the GitHub release of the same name with its
notes. Deploy a digest, not a tag, and verify it first:

```sh
gh attestation verify oci://ghcr.io/jimmylu/pp-cube-engine@sha256:<digest> \
  --repo jimmylu/pp-cube-engine
```

The digest is in the release notes. The image holds the binary and no shell,
and runs as a non-root user.

## The manifest

`deploy/kubernetes.yaml` holds a Deployment of three replicas, a Service and a
PodDisruptionBudget. Three things are yours to supply; the file says where.

- **The image**, by the digest you verified.
- **A secret `pp-cube-engine`** with `tls.crt`, `tls.key`, `read.token` and
  `admin.token`. The service listens beyond loopback, so it requires TLS and a
  read token. Tokens are read at start: rotate by updating the secret and
  restarting the Deployment.
- **The models.** Replicas read published model versions from shared storage
  and follow it. The manifest mounts a claim `pp-cube-engine-models`, read-only, at
  `/var/lib/pp-cube-engine/models`. For an S3-protocol bucket remove that volume
  and its mount, set `--model-store` to `s3://<bucket>/<prefix>`, and give the
  pod its region and, outside AWS, its endpoint as environment variables
  (`AWS_REGION`, `AWS_ENDPOINT_URL_S3`). Credentials come from the instance
  role or from `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` in a secret; the
  service takes none as an option. `--model-store` may be given twice to read
  a directory and a bucket at once.

And one `--data-source <name>=<dialect>` for every data source your models
name. A model on a data source without one is not offered.

What the manifest decides, and why:

| Setting | Value | Why |
| --- | --- | --- |
| Replicas | 3 | Two serve while one is replaced or lost |
| Rolling update | one unavailable, one extra | An upgrade never drops below two |
| Disruption budget | at least 2 | A node drain takes one replica at a time |
| Readiness and liveness | `GET /metrics` on 9100 | It answers once models are validated and the port is bound. The gRPC port is encrypted, which a Kubernetes gRPC probe cannot speak |
| Grace period | 45 s | Longer than `--drain-seconds 30`, so running requests finish |
| Root filesystem | read-only, non-root, no capabilities | The service writes nothing |
| CPU and memory | 2 cores, 512 Mi to 1 Gi | Starting values, not measured. One compilation uses one core; follow `--workers` |

The metrics port has no authentication. Keep it inside the cluster: scrape it
from there and do not expose it through the Service, which carries only gRPC.

**Balancing.** gRPC keeps one connection open, so a ClusterIP Service spreads
connections, not requests: a caller with one connection talks to one replica.
Callers should open several connections or balance on the client; a mesh or a
gRPC-aware proxy in front does it for them.

**Another platform.** Nothing here needs Kubernetes. Run the same image with
the same arguments under any supervisor that restarts it, sends SIGTERM to
stop it and waits 30 seconds; put a TCP or gRPC balancer in front.

## Upgrading

1. Read the release notes. "Breaking" lists what changes for callers.
2. Verify the new image and set its digest in the Deployment. Replicas are
   replaced one at a time; each drains its requests before it stops.
3. Watch `ppcube_build_info{kernel_version=…}` reach the new version on every
   replica, and `ppcube_requests_total` by outcome.

Two things are true during and after every upgrade:

- **Fingerprints change.** The kernel version is part of every fingerprint, so
  a cache keyed on fingerprints misses once after an upgrade even where the SQL
  is unchanged. While replicas of two versions serve, the same request can
  return two fingerprints; do not treat that as a difference in SQL.
- **Models are not touched.** Versions in the store are immutable and are read
  again by each new replica. A model version the new kernel rejects is logged
  as `model_store … rejected` and counted in
  `ppcube_model_store_rejected_versions`; the other versions are served. Check
  that metric after the first replica is up, before the rest follow. To hold a
  rollout after one replica, set `maxUnavailable: 0` and scale by hand, or use
  your platform's canary.

## Rolling back

Set the previous digest. That is all: there is no state to migrate, and the
store was not written. `kubectl rollout undo deployment/pp-cube-engine` does it.

A release is never withdrawn by moving its tag. If a version is bad, deploy the
one before it and wait for the next patch version.

Versions published to the store while the new kernel ran stay there. If one
uses something only the new kernel accepts, the old kernel rejects it by name
and serves the rest; retire it by removing its manifest, or leave it for the
fixed release.

## Losing a replica, or all of them

A replica that stops is replaced; the others keep serving and callers retry on
`UNAVAILABLE`. A full queue is `RESOURCE_EXHAUSTED`, a caller's signal to back
off or yours to add replicas. If the store cannot be reached, running replicas
keep the versions they have and `ppcube_model_store_last_scan_failed` is 1; a
replica that starts then stops at once, because it cannot list its store. So
do not restart replicas to fix a store.
