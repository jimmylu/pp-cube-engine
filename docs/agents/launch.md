# Launching an agent

For the owner. An agent session gets a token limited to this repository, never the owner's own token. The two token files are created by the owner outside the repository.

```sh
# Implementer (may push branches and open pull requests)
(set -a; . ~/.config/pp-cube-engine/implementer.env; set +a; omp)

# Reviewer (reads code, comments on pull requests, cannot push)
(set -a; . ~/.config/pp-cube-engine/reviewer.env; set +a; codex --sandbox read-only)
```

Neither token can approve `owner-approval`, change repository rules, or push files under `.github/workflows/`.

## Local guard per agent

| Agent | Guard | State |
| --- | --- | --- |
| Claude Code | `.claude/settings.json` runs `scripts/agent-guard.sh` before shell commands and file edits | Set up |
| Codex CLI | Start with `--sandbox workspace-write` to implement, `--sandbox read-only` to review | By launch flag; no project configuration file |
| omp | `.omp/hooks/pre/guard.ts`, discovered by its location, passes shell commands, writes and edits to `scripts/agent-guard.sh`; a call is refused when the guard cannot run | Set up |

The guard matches text, so it also refuses a command that merely quotes a blocked one, for example inside a prompt or a here-document. Put such text in a file.

The local guard prevents accidents. What actually protects `main` is the ruleset and the `owner-approval` gate, which hold for every agent.
