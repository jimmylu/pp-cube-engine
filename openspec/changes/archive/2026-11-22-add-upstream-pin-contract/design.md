# Design

## Context
The kernel's private fingerprint identity includes a constant upstream pin. The existing repository test compares it with HEAD's vendor/cube gitlink. Existing generic identity tests prove that changing the pin changes identity. The vendor and upstream CI steps enforce unmodified checkout/toolchain alignment.

## Goals / Non-Goals
Close the kernel-observable upstream identity contract. No pin move, copied upstream implementation or harness change.

## Decisions
Extend one existing repository test, retaining B0-DET-002 and adding B0-UPSTREAM-001. Read the checked-out vendor/cube HEAD via Git after verifying the gitlink, compare it with the kernel constant, then compile the existing neutral sales fixture via the public facade. Compare its complete fingerprint with identity calculated using the actual checkout SHA and current package version. Assert inequality when that pin input changes. One test covers the whole scenario; no tag-only adoption.

Keep existing source-archive and missing-Git skip behavior: no repository metadata means repository consistency cannot be verified. CI is the acceptance environment and supplies Git, gitlink and checkout. A present checkout that cannot be read is a failure. No silent fallback to a hardcoded checkout SHA. Vendor cleanliness stays with the existing mandatory vendor check, without duplicating its policy here.

Alternatives: adding a new public pin API expands the production surface unnecessarily; merely tagging the old test does not observe the actual compile result. The chosen extension uses the existing private identity seam without changing production code.

## Risks / Validation
Git reads the committed gitlink and checked-out submodule HEAD, so a checkout at another commit fails. Uncommitted upstream file edits remain enforced by the mandatory vendor check. Worktrees are supported because the existing metadata check accepts a .git file. Run the focused test and compile suite; fast before checkpoint and ci before final review. Test sensitivity by using another pin in the identity input; do not mutate vendor or repository state.

## Delivery
Transfer the entire seed requirement, preserve its purpose/id and delete the empty seed capability file. Verify, sync/archive after tests pass in the same PR. Get omp/Opus review and exact-head t0 before Ready; owner approves and merges. No runtime migration; rollback removes test/spec adoption.
