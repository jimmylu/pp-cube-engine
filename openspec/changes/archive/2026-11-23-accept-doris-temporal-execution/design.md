# Design

## Context

See proposal.md. The seed design archives a `(T2)` requirement only after its
tier is built and has passed for it, with the test in `pp-cube-conformance`
naming the id and not run by T0. Both now hold: the manifest cases live in the
conformance crate and the Doris container tier executes them on every delivery.

## Decisions

- **The cases are the test of record.** `// scenario: B0-DORIS-010` sits on the
  Doris case list. The T0 manifest test (B0-EXEC-001) only proves the manifest
  is emitted; it is not claimed as execution coverage. Alternative: keep an
  ignored Rust test as the id carrier, rejected because it duplicates the tier
  and starts a second engine.
- **Reword on transfer.** The seed text described a pending integration. The
  requirement keeps its name and scenario id; only the description changes to
  the implemented behavior. No id is reused or renumbered.
- **Seed file stays.** B0-DORIS-001 and B0-DORIS-002 remain in the seed delta,
  so only the moved requirement is deleted there.

## Risks / Trade-offs

- Traceability sees a comment, not an execution → acceptance rests on the tier
  result recorded against the head SHA in the pull request, not on the comment.
- The tier is local and on demand, not T0 → delivery runs it before every
  review; a tier failure leaves the pull request Draft.

## Migration Plan

Revert the pull request to restore the opt-in test. Evidence:
`scripts/check.sh ci`, `engines-container` and `doris-container` on the final head.
