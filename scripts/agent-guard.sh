#!/usr/bin/env bash
# Local guard against destructive mistakes during an agent session. It is a
# convenience, not the control: the controls are the ruleset on main and the
# owner-approval gate.
#
# Input: a Claude Code PreToolUse hook payload on stdin (JSON with tool_name
# and tool_input). Exit 2 blocks the tool call and shows stderr to the agent.
# Other agents can call it the same way: {"tool_name":"Bash","tool_input":{"command":"..."}}

set -euo pipefail

payload=$(cat)
tool=$(jq -r '.tool_name // ""' <<<"$payload")
command=$(jq -r '.tool_input.command // ""' <<<"$payload")
path=$(jq -r '.tool_input.file_path // .tool_input.notebook_path // ""' <<<"$payload")

block() { echo "agent-guard: blocked — $1. See AGENTS.md." >&2; exit 2; }

if [[ -n "$path" && "$path" =~ (^|/)vendor/cube(/|$) ]]; then
  block "vendor/ is read-only"
fi

[[ "$tool" == "Bash" && -n "$command" ]] || exit 0

rules=(
  'git[^|;&]*[[:space:]]push[^|;&]*[[:space:]](--force|--force-with-lease|-f)([[:space:]]|=|$)::force push'
  'git[^|;&]*[[:space:]]push[^|;&]*[[:space:]]\+?(HEAD:)?(refs/heads/)?main([[:space:]]|$)::push to main (open a pull request)'
  'git[^|;&]*[[:space:]]reset[^|;&]*[[:space:]]--hard::git reset --hard'
  'git[^|;&]*[[:space:]]clean[^|;&]*[[:space:]]-[a-zA-Z]*f::git clean -f'
  'git[^|;&]*[[:space:]]branch[^|;&]*[[:space:]]-D[[:space:]]::git branch -D'
  'gh[[:space:]]+pr[[:space:]]+merge::merging is the owner'"'"'s step'
  'gh[[:space:]]+api[^|;&]*(rulesets|environments|pending_deployments|actions/permissions)::changing or approving the gate'
  '(rm|mv|sed[[:space:]]+-i|tee|cp)[^|;&]*[[:space:]]vendor/cube::vendor/ is read-only'
  '>[[:space:]]*vendor/cube::vendor/ is read-only'
)

for rule in "${rules[@]}"; do
  pattern=${rule%%::*}
  reason=${rule##*::}
  if [[ "$command" =~ $pattern ]]; then block "$reason"; fi
done
exit 0
