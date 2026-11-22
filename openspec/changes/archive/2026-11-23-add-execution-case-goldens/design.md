# Design

## Context

See proposal.md. Goldens elsewhere are `insta` snapshots checked with
`INSTA_UPDATE=no`, reviewed with `cargo insta review`.

## Decisions

- **Snapshot the manifest, not a second copy of the queries.** The test renders
  each case's name, SQL and parameter values from the emitted manifest, so an
  executed case and its golden cannot drift apart. Alternative: goldens in
  `pp-cube-compile` repeating each query, rejected for that drift.
- **One snapshot file per dialect.** It reads as one document in manifest order;
  adding a case appends a block. Expected rows and fixtures stay out of it: they
  are literals in the case source, already reviewed there.
- **`insta`, not a hand-compared text file.** It keeps one golden workflow and
  the existing pending-snapshot check, at the cost of one `Cargo.lock` edge.
- **Travels alone.** The controller embeds changed files at base and head; a
  `Cargo.lock` change costs about 157 KB of its 500 KB review limit, so the
  dependency lands before the larger case change.

## Risks / Trade-offs

- Eleven blocks repeat SQL pinned by `pp-cube-compile` temporal goldens → accepted,
  to keep "every executed case has a golden" without exceptions.

## Migration Plan

Additive; revert to remove. Evidence: `scripts/check.sh ci` and both container tiers.
