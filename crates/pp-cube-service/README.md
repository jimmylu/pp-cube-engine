# Local gRPC service

Run validated model versions on a local socket:

```sh
cargo run -p pp-cube-service -- \
  --model v1=model.yaml --model v2=model-next.yaml \
  --listen 127.0.0.1:50051 --data-source warehouse=doris
```

`--model version=path` is repeatable; each file is UTF-8 YAML or JSON and every
one must be valid, or nothing starts. `--data-source name=dialect` is repeatable
and names the dialect (`presto`, `doris` or `druid`) that serves each data source; every
data source of the models needs one. Port zero chooses a free port. After
validation and binding, stdout prints `listening <address>` and, with a metrics
address, `metrics <address>`. Startup failure writes a diagnostic to stderr and
exits unsuccessfully.

## Deploying beyond loopback

```sh
pp-cube-service --model v1=model.yaml --data-source warehouse=doris \
  --listen 0.0.0.0:50051 --tls-cert server.pem --tls-key server.key \
  --token-file read.token --admin-token-file admin.token --allow-publish \
  --metrics-listen 127.0.0.1:9100
```

- A listen address other than loopback needs `--tls-cert`, `--tls-key` (PEM; an
  end-entity certificate, not a CA certificate) and `--token-file`.
- Tokens are the trimmed content of files, at least 16 visible ASCII characters,
  sent by callers as `authorization: Bearer <token>`. The read token admits
  Compile, ValidateModel and GetMeta. The admin token admits everything and is
  required for model management whenever tokens are in use. Rotate by restart.
- Without any token every caller is trusted; that is only allowed on loopback.
  Callers choose the security filters of a compilation, so a read token is
  already a trusted credential: give it to the system that enforces access, not
  to end users.

## Operating

| Option | Default | Meaning |
| --- | --- | --- |
| `--workers`, `--queue` | 2, 16 | Compilation threads and waiting requests; beyond that callers get `RESOURCE_EXHAUSTED` |
| `--compile-timeout-ms` | 10000 | How long a caller waits; then `DEADLINE_EXCEEDED`. The compilation itself cannot be interrupted and finishes on its worker |
| `--max-request-bytes` | 4194304 | Largest request, model uploads included |
| `--max-models` | 64 | Available versions; publishing beyond it fails until one is retired |
| `--model-store` | off | Published model versions to serve and follow: a directory or `s3://bucket/prefix`; repeatable; see below |
| `--model-refresh-seconds` | 30 | How often the store is read again; `0` reads it at startup only |
| `--drain-seconds` | 30 | Time for running requests to finish after SIGTERM or SIGINT |
| `--metrics-listen` | off | Address answering `GET /metrics` in the Prometheus text format |
| `--log` | `json` | `json` writes one line per request to stderr; `off` writes none |

Health: the standard `grpc.health.v1.Health/Check`, without a token, for the
empty service name or `ppcube.engine.v1.CubeEngine`. It reports `NOT_SERVING`
while the process drains.

Logs and metrics describe requests, never their content: operation, outcome,
duration, request id, model version, dialect and result fingerprint. They
contain no SQL, parameter or filter values, table names, model content or
tokens. Metrics: `ppcube_requests_total{rpc,outcome}`,
`ppcube_request_duration_seconds`, `ppcube_requests_in_flight`,
`ppcube_model_versions`, `ppcube_build_info`. The metrics address has no
authentication; keep it on an internal interface.

On SIGTERM or SIGINT the process stops accepting, finishes running requests and
exits successfully; if they outlast the drain time it exits unsuccessfully.

## Model versions

A version is identified by the service's digest of its typed content, `sha256:`
and 64 hex digits. The digest does not change with formatting, key order or
YAML versus JSON; ValidateModel returns it for a valid model.

### From shared storage

With `--model-store` every replica serves the versions a publisher writes to
shared storage and follows them while running. A store is a directory, such as
a NAS mount, or an S3-protocol bucket written `s3://bucket/prefix`; the option
may be given several times, and a process reads all its stores. The layout is
the same in both and keeps business lines and asset types apart:

```text
<business line>[/<sub line>[/<sub line>]]/models/<version>/manifest.json
                                                          /cubes/<cube>.yml
                                                          /views/<view>.yml
```

- **Business line**: one to three levels, lower case letters, digits, `_`, `-`.
- **Asset type**: `models` is the one this service reads. Anything under another
  name at that place is left alone, for other kinds of semantic assets.
- **Version**: letters, digits, `.`, `_`, `-`. A caller names a version by its
  business line and version: `property/motor/v12`.
- **Model**: the directory Cube recommends, one cube or view per file, each file
  named after what it defines in snake_case: `cubes/policies.yml` defines the
  cube `policies`. `.yml` and `.yaml` are read; any other file fails the version.
- **`manifest.json`**: `{"schema": 1}`, optionally with `"digest"`, the digest
  ValidateModel returns for the content. Write it last: a version is offered
  only once its manifest is there, and the digest, when given, must match.

Versions are immutable. One that is loaded is not read again; publish a change
as a new version. Removing a version's manifest retires it at the next pass.
`RetireModel` on a version of the store lasts until that pass: the store decides.

A store that cannot be listed stops the process at startup; later, a failed pass
changes nothing and the loaded versions stay. A version that cannot be offered
does not stop the others. It is logged once, and again when its reason changes,
as `{"event":"model_store","model_version":…,"outcome":"rejected","reason":…}`
with one of `layout`, `manifest`, `read`, `files`, `model`, `digest`,
`data-source`, `full`, and counted in
`ppcube_model_store_rejected_versions{store,reason}`;
`ppcube_model_store_scans_total{store,outcome}` and
`ppcube_model_store_last_scan_failed{store}` describe the passes. `store` is the
position of the store among the `--model-store` options, from 0. Run
ValidateModel on the content to see the issues.

Replicas read the store independently, so a new version is available on all of
them one refresh interval after its manifest is written, not at once.

Entries whose name starts with a dot are not read, in a directory or a bucket.

#### S3-protocol buckets

The service holds no secret and takes none as an option. It reads the variables
the AWS tools use:

| Variable | Meaning |
| --- | --- |
| `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_SESSION_TOKEN` | Access keys. Without them the instance role is used, through the instance metadata service (IMDSv2) |
| `AWS_REGION` or `AWS_DEFAULT_REGION` | The signing region; `us-east-1` when unset |
| `AWS_ENDPOINT_URL_S3` or `AWS_ENDPOINT_URL` | The endpoint of S3-protocol storage other than AWS, addressed path-style. Unset means AWS, addressed by bucket host name |
| `AWS_EC2_METADATA_SERVICE_ENDPOINT` | The metadata service, when it is not at its usual address |

The service lists the bucket under the prefix and reads objects; it needs
`s3:ListBucket` on the bucket and `s3:GetObject` under the prefix, and nothing
that writes. Web identity credentials (`AWS_WEB_IDENTITY_TOKEN_FILE`) are not
supported and are refused by name rather than taken for an instance role. A
failure is reported as what was asked and the HTTP status, never the server's
text. Use `https` endpoints beyond a trusted network: requests are signed, not
encrypted, over `http`.

### By request

With `--allow-publish`, versions can also be managed while serving:

- `PublishModel` validates a model and makes the version available. Versions
  are immutable: the same content again succeeds and creates nothing, different
  content is refused. State `expected_digest` to publish only that content.
- `RetireModel` stops offering a version; compilations already running on it
  finish. `ListModels` returns the available versions and digests.

Without the flag the three operations are refused, and at least one `--model`
is required. Published versions are held in memory: after a restart only the
`--model` files are available, so the host republishes (publishing is safe to
repeat). Anyone who can publish decides the SQL every caller receives: with
tokens in use it needs the admin token; without tokens enable it only on a
loopback socket the host controls.

## Contract

Clients use `ppcube.engine.v1.CubeEngine` from `proto/engine.proto`. Compile
names exactly one available version and supplies dialect, timezone and trusted
security filters. The service compiles SQL without executing it.

A compiled response carries the SQL, one typed value per placeholder to bind in
order, the output columns, the cubes and tables read, a fingerprint and the
engine session properties the SQL needs: apply every one before executing (on
Presto a query with an offset needs `offset_clause_enabled=true`). SQL is
returned only when the requested dialect serves every data source the query
reads. Every response for an available version names the kernel version, the
upstream pin and the model version and digest.

To embed the service in another host, use the library's `GrpcService` with
`Access`, `Limits` and `Telemetry`, and `HealthService`.
