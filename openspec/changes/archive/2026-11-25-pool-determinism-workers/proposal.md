# Keep the determinism workers for a whole test binary

## Why

A check took about twelve minutes, of which about nine were the compile
tests: every successful compilation in them started four fresh processes to
compare results (B0-DET-001), and the B1 changes added many golden cases.
Each pull request runs the suite several times, and Druid will add a dialect's
worth of cases. This is the batched test rework of AGENTS.md rule 9 (route
item G11), limited to what makes the checks slow.

## What Changes

- The four comparison processes are started once for a test binary and kept:
  every compilation of the binary is sent to each as one line and must come
  back byte for byte. Before: four new processes for every compilation.
- The comparison is unchanged, and is still made in four other processes.
  Kept alive, they also show state leaking from one compilation to the next,
  which fresh processes could not.
- No product code changes. No golden changes.

Non-goals: merging the helpers repeated across test files; a faster test
runner or skipped checks in the controller, which are harness changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities
- `determinism`: how the comparison processes are started (B0-DET-001).

## Impact

Paths: `crates/pp-cube-compile/tests/support/mod.rs`, one spec and this
change folder. Callers: none.
