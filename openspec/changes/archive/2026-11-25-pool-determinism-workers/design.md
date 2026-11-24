# Design

## Measured

On the same machine, debug build, `cargo test -p pp-cube-compile`:

| | Before | After |
| --- | --- | --- |
| Eleven test binaries that replay, test time | about 536 s (18 to 110 s each) | about 57 s (4 to 6 s each) |
| Whole `scripts/check.sh ci` | 700 to 760 s | recorded on the pull request |

Before figures are from the `ci` run of the previous change; everything else
in a check was under three minutes.

## Decisions

- **A pool behind a mutex, one line a compilation.** The first compilation of
  a test binary starts four copies of the binary running only the worker
  test. Each later compilation is written to every worker's input as one JSON
  line and its answer read back. Tests of a binary run on several threads;
  the mutex gives the workers one compilation at a time, which costs nothing
  measurable. The workers end when the binary does: their input closes.
- **Still other processes, no longer fresh for each case.** What the
  requirement guards is that nothing process-local (hash seeds, addresses,
  iteration order) reaches the output. Four processes with their own seeds
  still show that. A process that has compiled a hundred other queries first
  additionally shows state carried between compilations. What is lost: a
  case is no longer the first thing its comparison process compiles. The
  test process itself still compiles every case first in some test.
- **Checked by mutation.** With a worker made to answer differently on its
  third compilation, every test binary's replay failed in all four processes;
  the change was then reverted.

## Risks / Trade-offs

- A worker that dies takes the rest of its binary's replays with it → each
  fails with "ended without a result", naming the query.

## Migration Plan

Tests only. Evidence: `scripts/check.sh ci`, remote `t0`.
