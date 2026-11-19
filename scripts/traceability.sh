#!/usr/bin/env bash
# Links spec scenarios to tests.
#
# A scenario id is the first word of a scenario title in openspec/specs/:
#     #### Scenario: B0-AGG-003 Sum over one cube
# A test covers a scenario by naming its id in a comment or attribute:
#     // scenario: B0-AGG-003
#
# Fails when a scenario has no test, when a test names an unknown id, or when
# a scenario title has no id.

set -euo pipefail
cd "$(dirname "$0")/.."

ID_RE='[A-Z][A-Z0-9]*(-[A-Z0-9]+)+'
status=0

titles=$(grep -rhE '^#### Scenario:' openspec/specs 2>/dev/null || true)

without_id=$(grep -Ev "^#### Scenario: +$ID_RE( |$)" <<<"$titles" | sed '/^$/d' || true)
if [[ -n "$without_id" ]]; then
  printf 'Scenario titles without an id:\n%s\n\n' "$without_id"; status=1
fi

specified=$(sed -nE "s/^#### Scenario: +($ID_RE)( .*)?$/\1/p" <<<"$titles" | sort)
duplicated=$(uniq -d <<<"$specified")
if [[ -n "$duplicated" ]]; then
  printf 'Scenario ids used more than once:\n%s\n\n' "$duplicated"; status=1
fi
specified=$(sort -u <<<"$specified")

tested=$(grep -rhoE "scenario: *$ID_RE" crates --include='*.rs' 2>/dev/null \
  | sed -E 's/^scenario: *//' | sort -u || true)

untested=$(comm -23 <(sed '/^$/d' <<<"$specified") <(sed '/^$/d' <<<"$tested"))
unknown=$(comm -13 <(sed '/^$/d' <<<"$specified") <(sed '/^$/d' <<<"$tested"))

if [[ -n "$untested" ]]; then printf 'Scenarios without a test:\n%s\n\n' "$untested"; status=1; fi
if [[ -n "$unknown" ]]; then printf 'Tests naming an unknown scenario:\n%s\n\n' "$unknown"; status=1; fi

count() { sed '/^$/d' <<<"$1" | wc -l | tr -d ' '; }
echo "traceability: $(count "$specified") scenarios specified, $(count "$tested") named by tests"
exit $status
