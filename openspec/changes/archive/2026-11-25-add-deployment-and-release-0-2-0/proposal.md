# Proposal

## Why
The kernel is to go live as three replicas, and until now there was no released image, no description of a deployment and no word on upgrading or rolling back. The release workflow exists (#103) and publishes to this repository's own registry; it needs a version to release and something to run it with.

## What Changes
- `deploy/kubernetes.yaml`: a Deployment of three replicas, a Service and a PodDisruptionBudget, reading models from shared storage, with TLS and tokens from a secret.
- `docs/deployment.md`: what the manifest decides and why, the S3 variant, upgrading, rolling back, and what happens when a replica or the store is lost.
- The workspace version becomes 0.2.0 and the changelog's unreleased entries become its section, so the owner can tag `v0.2.0`.
- A test starts the service with exactly the manifest's arguments.

## Capabilities
### New Capabilities
None.
### Modified Capabilities
- `service-api`: the shipped deployment is part of the contract: its arguments start a serving replica.

## Impact
`deploy/`, `docs/deployment.md`, `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`, one test in `crates/pp-cube-service`, links in the guide and README. No SQL changes. Every fingerprint changes, as with any version: the kernel version is part of it. No golden holds a fingerprint.
