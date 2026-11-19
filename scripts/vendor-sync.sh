#!/usr/bin/env bash
# Checks out vendor/cube at the commit this repository pins, as a partial,
# sparse checkout: only the planner workspace and the one crate it needs from
# the cubesql workspace. A plain `git submodule update` would fetch all of Cube.

set -euo pipefail
cd "$(dirname "$0")/.."

VENDOR="vendor/cube"
SPARSE=(rust/cube rust/cubesql/cubeclient)

want=$(git ls-files -s "$VENDOR" | awk '{print $2}')
[[ -n "$want" ]] || { echo "vendor-sync: $VENDOR is not a submodule in this commit" >&2; exit 1; }
url=$(git config -f .gitmodules "submodule.$VENDOR.url")

git submodule init -q "$VENDOR"

# An empty prefix means vendor/cube is its own repository; a non-empty one
# means git fell through to this repository, so nothing is checked out yet.
if [[ ! -d "$VENDOR" || -n "$(git -C "$VENDOR" rev-parse --show-prefix 2>/dev/null || echo none)" ]]; then
  mkdir -p "$VENDOR"
  git -C "$VENDOR" init -q
  git -C "$VENDOR" remote add origin "$url"
  git -C "$VENDOR" config remote.origin.promisor true
  git -C "$VENDOR" config remote.origin.partialclonefilter blob:none
fi

git -C "$VENDOR" sparse-checkout set --cone "${SPARSE[@]}"

if [[ "$(git -C "$VENDOR" rev-parse -q --verify HEAD 2>/dev/null || true)" != "$want" ]]; then
  git -C "$VENDOR" fetch -q --depth 1 --filter=blob:none origin "$want"
  git -C "$VENDOR" checkout -q --detach "$want"
fi

git submodule absorbgitdirs "$VENDOR" >/dev/null 2>&1 || true

have=$(git -C "$VENDOR" rev-parse HEAD)
[[ "$have" == "$want" ]] || { echo "vendor-sync: $VENDOR is at $have, expected $want" >&2; exit 1; }
echo "vendor-sync: $VENDOR at $want"
