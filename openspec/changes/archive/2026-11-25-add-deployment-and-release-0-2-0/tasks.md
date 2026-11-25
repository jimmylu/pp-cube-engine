# Tasks

## 1. Deployment

- [x] 1.1 `deploy/kubernetes.yaml`; test B1-API-004 starts the service with its arguments.
- [x] 1.2 `docs/deployment.md`: the manifest, upgrading, rolling back, losing a replica or the store; links from the guide and README.

## 2. Version

- [x] 2.1 Workspace version 0.2.0, `Cargo.lock`, changelog section; verify with `scripts/release.sh check v0.2.0`.

## Workflow follow-up

- Archived in the same commit as the implementation.
- After the merge the owner tags `v0.2.0`; `docs/agents/release.md` has the commands. The `publish` job runs for the first time then.
