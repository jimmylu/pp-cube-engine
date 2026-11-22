#!/usr/bin/env bash
# The single definition of "passing". Agents run it locally; CI runs the same
# script. Modes are described in docs/agents/verification.md.
#
#   scripts/check.sh fast          before every commit
#   scripts/check.sh ci            before a pull request; this is the t0 check
#   scripts/check.sh conformance   comparison with Cube Core (not built yet)
#   scripts/check.sh engines       Presto smoke case on a prepared engine
#
# BASE_REF (default origin/main) is the branch a change is compared against.

set -euo pipefail
cd "$(dirname "$0")/.."

export OPENSPEC_TELEMETRY=0
export CARGO_TERM_COLOR="${CARGO_TERM_COLOR:-always}"
# Snapshot tests never write or accept snapshots during a check.
export INSTA_UPDATE=no

MODE="${1:-}"
BASE_REF="${BASE_REF:-origin/main}"
# The OpenSpec CLI version the specs and project skills were written with.
# .github/workflows/t0.yml installs the same version.
OPENSPEC_VERSION="2.1.0"
VENDOR="vendor/cube"
UPSTREAM_WS="$VENDOR/rust/cube"

# Paths that make up the harness. Keep in step with the risk:harness rule in
# .github/workflows/gate.yml.
HARNESS_RE='^(\.github/|scripts/|\.agents/|\.claude/|\.codex/|\.omp/|docs/agents/|(.*/)?AGENTS\.md$|CLAUDE\.md$|deny\.toml$|clippy\.toml$|rustfmt\.toml$|\.rustfmt\.toml$|typos\.toml$|_typos\.toml$|taplo\.toml$|\.taplo\.toml$|rust-toolchain\.toml$)'

PASSED=()
SKIPPED=()
START=$SECONDS

step() { printf '\n\033[1m▸ %s\033[0m\n' "$1"; }
ok()   { PASSED+=("$1"); }
skip() { SKIPPED+=("$1"); printf '  skipped: %s\n' "$1"; }
fail() { printf '\n\033[1mcheck %s: FAILED\033[0m at "%s" — %s\n' "$MODE" "$1" "$2" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || fail "tools" "$1 is not installed (see docs/agents/verification.md)"; }

summary() {
  local line="check $MODE: PASSED — ${#PASSED[@]} steps, ${#SKIPPED[@]} skipped, $((SECONDS - START))s"
  if (( ${#SKIPPED[@]} )); then line+=" (skipped: $(IFS=';'; echo "${SKIPPED[*]}"))"; fi
  line+=" @ $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- 2>/dev/null || echo '+dirty')"
  printf '\n\033[1m%s\033[0m\n' "$line"
}

# The commit this change branched from, or empty when it cannot be found.
merge_base() { git merge-base HEAD "$BASE_REF" 2>/dev/null || true; }

# Files changed relative to the base, including uncommitted and untracked ones.
changed_files() {
  local base; base=$(merge_base)
  {
    if [[ -n "$base" ]]; then git diff --name-only "$base"; fi
    git ls-files --others --exclude-standard
  } | sort -u
}

check_vendor() {
  step "vendor: upstream checkout present and untouched"
  [[ -f "$UPSTREAM_WS/Cargo.toml" ]] || fail "vendor" "$VENDOR is not checked out; run scripts/vendor-sync.sh"
  local want have
  want=$(git ls-files -s "$VENDOR" | awk '{print $2}')
  have=$(git -C "$VENDOR" rev-parse HEAD)
  [[ "$want" == "$have" ]] || fail "vendor" "$VENDOR is at $have but the repository pins $want; run scripts/vendor-sync.sh"
  [[ -z "$(git -C "$VENDOR" status --porcelain)" ]] || fail "vendor" "files under $VENDOR were modified; vendor/ is read-only"
  ok vendor
}

check_upstream_alignment() {
  step "upstream: toolchain and [patch] match the pinned commit"
  local ours theirs
  ours=$(grep -E '^channel' rust-toolchain.toml)
  theirs=$(grep -E '^channel' "$UPSTREAM_WS/rust-toolchain.toml")
  [[ "$ours" == "$theirs" ]] || fail "upstream" "rust-toolchain.toml has '$ours', upstream has '$theirs'"
  patch_section() { awk '/^\[patch/ {on=1} /^\[/ && !/^\[patch/ {on=0} on && NF && !/^#/' "$1"; }
  diff <(patch_section Cargo.toml) <(patch_section "$UPSTREAM_WS/Cargo.toml") >/dev/null \
    || fail "upstream" "the [patch] section in Cargo.toml differs from $UPSTREAM_WS/Cargo.toml"
  ok upstream
}

check_format() {
  step "format"
  # Without --all: that flag also formats path dependencies, which are in vendor/.
  cargo fmt -- --check || fail "format" "run: cargo fmt"
  ok format
}

check_clippy() {
  step "clippy (warnings are errors)"
  cargo clippy --workspace --all-targets --locked -- -D warnings || fail "clippy" "fix the lints above"
  ok clippy
}

# Workspace crates touched by this change; empty means "test everything".
changed_crates() {
  local files; files=$(changed_files)
  if grep -Evq '^crates/[^/]+/' <<<"$files"; then return 0; fi
  sed -nE 's#^crates/([^/]+)/.*#\1#p' <<<"$files" | sort -u
}

check_tests() { # all | changed
  local args=(--workspace)
  if [[ "$1" == changed ]]; then
    local crates; crates=$(changed_crates)
    if [[ -n "$crates" ]]; then
      args=()
      while read -r c; do [[ -d "crates/$c" ]] && args+=(-p "$c"); done <<<"$crates"
      (( ${#args[@]} )) || args=(--workspace)
    fi
  fi
  step "tests (${args[*]})"
  cargo test "${args[@]}" --locked || fail "tests" "a test failed"
  if [[ -n "$(git ls-files --others --exclude-standard --ignored -- '*.snap.new' '*.pending-snap')" ]]; then
    fail "tests" "pending snapshots exist; review them with cargo insta, do not commit them"
  fi
  ok tests
}

check_lockfile() {
  step "lock file: committed and current"
  [[ -f Cargo.lock ]] || fail "lockfile" "Cargo.lock is missing"
  cargo metadata --locked --format-version 1 >/dev/null || fail "lockfile" "Cargo.lock is out of date"
  ok lockfile
}

check_deny() {
  step "cargo deny"
  need cargo-deny
  cargo deny --locked check || fail "deny" "see the advisories, bans, licences or sources above"
  ok deny
}

check_text() {
  step "typos"
  need typos
  typos || fail "typos" "fix the spelling, or add a justified entry to typos.toml in a harness change"
  ok typos
  step "taplo"
  need taplo
  taplo fmt --check || fail "taplo" "run: taplo fmt"
  ok taplo
}

check_workflows() {
  step "actionlint"
  if command -v actionlint >/dev/null 2>&1; then
    actionlint || fail "actionlint" "fix the workflow errors above"
    ok actionlint
  else
    skip "actionlint not installed"
  fi
}

check_secrets() {
  step "gitleaks"
  if command -v gitleaks >/dev/null 2>&1; then
    gitleaks dir . --no-banner --redact >/dev/null 2>&1 || fail "gitleaks" "a secret was found; run: gitleaks dir . --redact"
    ok gitleaks
  else
    skip "gitleaks not installed"
  fi
}

check_openspec() {
  step "openspec validate"
  if [[ ! -d openspec ]]; then skip "openspec not set up yet"; return; fi
  need openspec
  local have; have=$(openspec --version)
  [[ "$have" == "$OPENSPEC_VERSION" ]] || fail "openspec" "openspec $have is installed; this repository uses $OPENSPEC_VERSION"
  openspec validate --all --strict --no-interactive || fail "openspec" "fix the spec errors above"
  ok openspec
}

check_traceability() {
  step "traceability: every scenario has a test, every test names a real scenario"
  if [[ ! -d openspec/specs ]]; then skip "no specs yet"; return; fi
  scripts/traceability.sh || fail "traceability" "see the list above"
  ok traceability
}

check_harness_only() {
  step "harness-only rule"
  local base; base=$(merge_base)
  if [[ -z "$base" ]]; then skip "no $BASE_REF to compare with"; return; fi
  # The change that introduces the harness is the one exception.
  if ! git cat-file -e "$base:scripts/check.sh" 2>/dev/null; then
    skip "harness-only rule (the base has no harness yet)"; return
  fi
  local files harness other
  files=$(changed_files)
  harness=$(grep -E "$HARNESS_RE" <<<"$files" || true)
  other=$(grep -Ev "$HARNESS_RE" <<<"$files" | sed '/^$/d' || true)
  if [[ -n "$harness" && -n "$other" ]]; then
    printf 'Harness files:\n%s\n\nOther files:\n%s\n' "$harness" "$other"
    fail "harness-only" "harness changes travel alone; split this change"
  fi
  ok harness-only
}

case "$MODE" in
  fast)
    check_vendor
    check_format
    check_clippy
    check_tests changed
    ;;
  ci)
    step "delivery controller tests"
    python3 -B -m unittest discover -s scripts/tests || fail "delivery-tests" "fix controller regressions"
    ok delivery-tests
    check_vendor
    check_upstream_alignment
    check_lockfile
    check_format
    check_clippy
    check_tests all
    check_deny
    check_text
    check_workflows
    check_secrets
    check_openspec
    check_traceability
    check_harness_only
    ;;
  engines)
    python3 scripts/engines/presto.py
    exit $?
    ;;
  conformance)
    echo "check $MODE: not built yet (it arrives with the first conformance work)." >&2
    exit 2
    ;;
  *)
    echo "usage: scripts/check.sh fast|ci|conformance|engines" >&2
    exit 64
    ;;
esac

summary
