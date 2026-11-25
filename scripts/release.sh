#!/usr/bin/env bash
# What a release needs to know before anything is built or published.
#
#   scripts/release.sh version            the workspace version, from Cargo.toml
#   scripts/release.sh check <tag>        the tag is v<workspace version>, the changelog
#                                         has that version's section, and the tagged
#                                         commit is on main
#   scripts/release.sh notes <version>    that section of CHANGELOG.md, for the release
#   scripts/release.sh image <repo>       the image name of a repository: its own
#                                         container registry, lower case
#   scripts/release.sh build <image> <version> <arch> <revision> [push]
#                                         builds the service image for linux/<arch>
#                                         with its SBOM and provenance; with `push`
#                                         it is pushed by digest, without a tag, and
#                                         the digest is printed; without, it is only
#                                         built
#
# Only `build … push` publishes, and only an untagged image. The release workflow
# calls it; see docs/agents/release.md.

set -euo pipefail
cd "$(dirname "$0")/.."

fail() { echo "release: FAILED — $1" >&2; exit 1; }

version() {
  # The one version every crate inherits.
  awk '/^\[workspace\.package\]/ { found = 1; next }
       /^\[/ { found = 0 }
       found && /^version = "/ { gsub(/^version = "|"$/, ""); print; exit }' Cargo.toml
}

notes() {
  # The lines under "## <version>", up to the next version's heading.
  awk -v heading="## $1" '
    $0 == heading || index($0, heading " ") == 1 { found = 1; next }
    found && /^## / { exit }
    found { print }' CHANGELOG.md | sed -e '/./,$!d'
}

case "${1:-}" in
  version)
    found="$(version)"
    [[ "$found" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "Cargo.toml has no workspace version of the form X.Y.Z"
    echo "$found"
    ;;
  check)
    tag="${2:-}"
    found="$(version)"
    [[ "$found" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "Cargo.toml has no workspace version of the form X.Y.Z"
    [[ "$tag" == "v$found" ]] || fail "the tag must be v$found, the workspace version; move the version in a product change first"
    [[ -n "$(notes "$found")" ]] || fail "CHANGELOG.md has no section '## $found' with content"
    # A release is cut from what was reviewed and merged, never from a side branch.
    if [[ "${RELEASE_SKIP_MAIN_CHECK:-}" != "1" ]]; then
      git merge-base --is-ancestor HEAD origin/main 2>/dev/null \
        || fail "the tagged commit is not on main"
    fi
    echo "release: v$found"
    ;;
  notes)
    [[ -n "${2:-}" ]] || fail "usage: scripts/release.sh notes <version>"
    found="$(notes "$2")"
    [[ -n "$found" ]] || fail "CHANGELOG.md has no section '## $2' with content"
    echo "$found"
    ;;
  image)
    [[ "${2:-}" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]] || fail "usage: scripts/release.sh image <owner>/<repository>"
    echo "ghcr.io/$(echo "$2" | tr '[:upper:]' '[:lower:]')"
    ;;
  build)
    image="${2:-}" release="${3:-}" arch="${4:-}" revision="${5:-}" push="${6:-}"
    [[ "$image" =~ ^ghcr\.io/[a-z0-9_.-]+/[a-z0-9_.-]+$ ]] || fail "the image must be a ghcr.io name in lower case"
    [[ "$release" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "the version must be X.Y.Z"
    [[ "$arch" == amd64 || "$arch" == arm64 ]] || fail "the architecture must be amd64 or arm64"
    [[ "$revision" =~ ^[0-9a-f]{40}$ ]] || fail "the revision must be a full commit id"
    [[ -z "$push" || "$push" == push ]] || fail "the last argument, if any, must be push"
    [[ -f vendor/cube/rust/cube/Cargo.toml ]] || fail "the pinned upstream is not checked out; run scripts/vendor-sync.sh"
    work="$(mktemp -d)"
    trap 'rm -rf "$work"' EXIT
    if [[ "$push" == push ]]; then
      output="type=image,name=$image,push-by-digest=true,name-canonical=true,push=true"
    else
      output="type=oci,dest=$work/image.tar"
    fi
    # The labels tie the image to this repository and commit; the SBOM and the
    # provenance are stored with it in the registry.
    docker buildx build . --platform "linux/$arch" \
      --label "org.opencontainers.image.source=https://github.com/${image#ghcr.io/}" \
      --label "org.opencontainers.image.version=$release" \
      --label "org.opencontainers.image.revision=$revision" \
      --label "org.opencontainers.image.licenses=Apache-2.0" \
      --sbom=true --provenance=mode=max \
      --output "$output" --metadata-file "$work/metadata.json" >&2 || fail "build"
    digest="$(python3 -c 'import json, sys; print(json.load(open(sys.argv[1])).get("containerimage.digest", ""))' "$work/metadata.json")"
    [[ "$digest" =~ ^sha256:[0-9a-f]{64}$ ]] || fail "the build reported no digest"
    echo "$digest"
    ;;
  *)
    echo "usage: scripts/release.sh version | check <tag> | notes <version> | image <owner>/<repository> | build <image> <version> <arch> <revision> [push]" >&2
    exit 64
    ;;
esac
