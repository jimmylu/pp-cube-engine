# Release

A release is a tag `v<version>` on `main`. Pushing it runs the `release` workflow,
which publishes the service image to this repository's own container registry
and creates the GitHub release. The owner pushes the tag; an agent prepares
everything up to it and never tags.

## What a release is

- **The image** `ghcr.io/<owner>/<repository>:<version>`, for linux/amd64 and
  linux/arm64, built from the tagged commit by the repository's `Dockerfile` on a
  runner of each architecture. The name is derived from the repository; no
  workflow or script holds it. There is no `latest`: a deployment names a
  version, or better its digest.
- **Its SBOM and provenance**, stored with the image in the registry
  (`--sbom=true --provenance=mode=max`), and a provenance attestation signed
  through GitHub's Sigstore instance (`actions/attest-build-provenance`).
- **The GitHub release** `v<version>`, whose notes are that version's section
  of `CHANGELOG.md` followed by the image reference with its digest.

A published version is never replaced: the workflow fails if the version tag
already exists in the registry.

## Before the tag: a product change

The workflow refuses a tag unless all three hold (`scripts/release.sh check`):

1. the tag is `v` plus the workspace version of `Cargo.toml`;
2. `CHANGELOG.md` has a section `## <version>` with content;
3. the tagged commit is on `main`.

So a release starts with a product change that sets `version` under
`[workspace.package]`, moves what is under `## Unreleased` to `## <version>`,
and updates `Cargo.lock`. The kernel version is part of every fingerprint:
fingerprints change with the release, and the changelog says so. That change
is reviewed and merged like any other. Then the owner tags the merge commit:

```sh
git switch main && git pull --ff-only
git tag v<version> && git push origin v<version>
```

## What runs

| Job | Does | Publishes |
| --- | --- | --- |
| `prepare` | `scripts/release.sh check <tag>`; derives version and image name | nothing |
| `image-amd64`, `image-arm64` | `scripts/release.sh build … push`: the image of one architecture, pushed by digest without a tag | an untagged image |
| `publish` | joins both under `<version>`, signs the provenance, creates the release | the version tag, the attestation, the release |

Only `publish` has write access to contents, attestations and the OIDC token.
The engine and build checks are not repeated: the tagged commit is on `main`,
where `t0` ran for its pull request and `image` and `aarch64` ran on the push.
Check that they are green on that commit before tagging.

## Rehearsal

A pull request that changes `.github/workflows/release.yml` or
`scripts/release.sh` runs `prepare` and both image jobs without publishing: the
images are built with their SBOM and provenance and discarded, and `publish`
is skipped. That is the test of a change to the release; `scripts/dev deliver`
does not wait for it, so read its result on the pull request.
`scripts/tests/test_release.py` covers the script and the workflow's publishing
conditions and runs in `ci`.

Locally, with Docker and a `docker-container` builder:

```sh
scripts/vendor-sync.sh
docker buildx create --name release-check --use
scripts/release.sh build ghcr.io/example/repo 0.0.0 arm64 "$(git rev-parse HEAD)"
docker buildx rm release-check
```

## Using and verifying a release

```sh
docker pull ghcr.io/<owner>/<repository>:<version>
gh attestation verify oci://ghcr.io/<owner>/<repository>@<digest> --repo <owner>/<repository>
```

The first image creates the package. Its visibility is a setting of the package
on GitHub, not of this repository: the owner makes it public, or grants the
deployment's credentials read access, once.

## When it fails

- `prepare` failed: nothing was published. Fix the version or changelog in a
  product change, delete the tag, and tag the new commit.
- An image job failed: an untagged image of the other architecture may exist
  in the registry; it is unreachable by tag and can be left or deleted. Re-run
  the failed job.
- `publish` failed after the version tag was created: re-running stops at
  "already published". The owner decides: delete that version in the registry
  and re-run, or release the next patch version.

A bad release is not withdrawn by moving its tag. Deployments roll back to the
previous version; the fix is a new version.
