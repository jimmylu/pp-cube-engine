# Model store: versions from shared storage

## Why

Going live means three replicas, with the semantic layer publishing models
(route item G2, `review.md` of `archive/2026-11-24-add-insurance-scenario`).
Today a version published at run time lives in one process's memory: the other
replicas do not have it and a restart loses it. The owner decided on
2026-11-24 that replicas read models from shared storage, a NAS directory and
in enterprise environments S3-protocol object storage; that the layout
isolates business lines (up to three levels) and asset types, leaving room for
other semantic assets; and that model files follow Cube's recommended naming.

## What Changes

- **Model directories.** A model may be a directory, one cube per file under
  `cubes/` and one view per file under `views/`, each named after what it
  defines. Assembling them is pure and gives the model one document would.
- **A store layout:** `<business line>[/<sub>[/<sub>]]/models/<version>/`,
  with `manifest.json` written last to mark a version complete, optionally
  carrying the expected digest. Other asset types under a business line are
  left alone.
- **The service reads a store** given with `--model-store <directory>`: a first
  pass before it announces itself, then one every `--model-refresh-seconds`.
  Complete versions are offered, versions whose manifest is gone are retired,
  and a version that cannot be offered is reported by reason without stopping
  the others. A caller names a version as `<business line>/<version>`.
- **A store is a trait** of two operations, list and read, so that an
  S3-protocol bucket is a second implementation of the same layout.
- Baseline level: B1. No golden changes; no compiled SQL changes.

Non-goals: **the S3-protocol store itself**, which is the next change: it
brings an HTTP and signing stack into the service and needs an S3-protocol
server in the tiers to be tested honestly, a harness change. Also not here:
other asset types; a wire field for the business line; writing to the store;
change notification instead of polling.

## Capabilities

### New Capabilities

None.

### Modified Capabilities
- `contract-model`: model directories (B1-MODEL-004).
- `service-api`: the store's layout, loading, following and the process (B1-STORE-001 to -003, B1-API-002).

## Impact

Paths: `crates/pp-cube-model` (new `assemble.rs`), `crates/pp-cube-service`
(new `store.rs`, `main.rs`, README), tests of both crates, `docs/guide.md`,
`CHANGELOG.md`, two specs and this change folder. Callers: two new options;
nothing existing changes. No new dependency.
