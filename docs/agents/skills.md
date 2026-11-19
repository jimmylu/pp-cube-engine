# Skills

Project skills live in `.agents/skills/` and are the same for every agent. There are none yet: each is written after its procedure has been done once by hand.

Planned: `pp-conformance-case`, `pp-dialect-port`, `pp-upstream-bump`, `pp-change-review`.

## Global skills the workflow relies on

These are installed on the owner's machine, not in this repository, and can change independently of it. An agent that does not have one follows the plain instruction in the right-hand column.

| Skill | Used for | Without it |
| --- | --- | --- |
| `tdd` | Implementing each task | Write the failing test first, then the code |
| `rust-best-practices` | Writing and reviewing Rust | Idiomatic Rust; `clippy` is the floor |
| `code-review` | The reviewer role | Review against the change's spec delta and `AGENTS.md` |
| `diagnosing-bugs` | A failing check | Reproduce, isolate, then fix |
| `source-driven-development` | Adapter and dialect work that must follow upstream exactly | Read the upstream source at the pinned commit before writing |
| `pr` | Pull request bodies | Fill in the pull request template |
| `grilling` | Stress-testing a proposal before Gate 1 | — |
| `handoff` | Session end | Update `tasks.md` and `notes.md` |
| `documentation-and-adrs`, `domain-modeling` | Decision records; the glossary | — |

## Not used here

Skills that create a second spec or ticket system beside the change folders (`to-spec`, `to-tickets`, `implement-spec`, `wayfinder`), and `setup-pre-commit`.
