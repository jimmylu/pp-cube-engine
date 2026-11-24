# S3-protocol model store

## Why

The owner decided on 2026-11-24 that replicas read models from a NAS
directory and, in enterprise environments, from S3-protocol object storage,
both supported at once, with credentials from the standard environment
variables and the instance role. `add-model-store` delivered the layout, the
store trait and the directory store. This completes route item G2.

## What Changes

- **An S3-protocol bucket is a store:** `--model-store s3://bucket/prefix`,
  with the same layout as a directory. Listing is paginated; requests are
  signed; folder markers and hidden entries are left out.
- **Credentials** from `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` (with
  `AWS_SESSION_TOKEN`), or without them from the instance role through the
  metadata service (IMDSv2). Region, a custom endpoint and the metadata
  endpoint from their standard variables. The service holds no secret.
- **Several stores at once:** `--model-store` is repeatable, directories and
  buckets together. Store metrics gain a `store` label, the store's position.
- **A rejected version is logged once**, and again when its reason changes.
  Before, every pass logged it: found when running against a real server.
- Two dependencies: `rusty-s3`, which signs and does no IO, and `ureq`, a
  blocking HTTP client on the rustls and ring already in the lockfile.
- Baseline level: B1. No golden changes; no compiled SQL changes.

Non-goals: web identity credentials (refused by name); writing to a bucket;
change notification; an S3-protocol server in the tiers, which is a harness
change and follows.

## Capabilities

### New Capabilities

None.

### Modified Capabilities
- `service-api`: the S3-protocol store, its credentials, several stores (B1-STORE-004, B1-STORE-005, B1-API-003); rejections logged once (B1-STORE-003).

## Impact

Paths: `crates/pp-cube-service` (new `s3.rs`, `store.rs`, `main.rs`,
`Cargo.toml`, README), its tests, `Cargo.lock`, `docs/guide.md`,
`CHANGELOG.md`, one spec and this change folder. Callers: the metrics
`ppcube_model_store_*` gain the label `store`; they were added in the
previous change and are in no release.
