# Presto Tokyo day execution

Branch: harness/presto-time, from main after merged PR #43.

## Design

The next B0 engine gap is temporal semantics. Execute the existing Tokyo day golden
unchanged, alongside aggregate/text-filter checks. Extend the fixture with UTC
sold_at timestamps surrounding the golden's exact lower and upper bounds. Keep
previous aggregate/filter expected results stable by using a zero-amount upper
outsider. Expected temporal rows are October 1=20 and October 31=7, derived from
the four fixture instants and Tokyo's offset rather than copied from engine output.

The two existing Text date parameters travel in order through the prepared-header
protocol. Support one or two Text snapshot parameters only; reject numeric, null,
mixed and larger lists. Do not add a generic Rust Debug decoder. Reuse all HTTP,
container lifecycle and delivery bounds. Scope is harness-only, with no product,
upstream, golden or baseline changes. No full B0/T2, DST or live compilation claim.

Alternatives: numeric binding would not validate timezone or date boundaries;
starting another engine adds provisioning cost before Presto time is verified.
A separate live compilation executable belongs in a product change.

## Tasks

- [x] Confirm #43 merge and create isolated branch from main.
- [x] Reuse the unchanged Tokyo day golden and choose independent boundary data.
- [x] Support ordered one/two Text prepared parameters and reject other shapes.
- [x] Extend fixture and suite while preserving earlier aggregate/filter oracles.
- [x] Regress parameter order, wrong time buckets, range leakage and totals.
- [ ] Verify final-head CI and real local Docker before review.
- [ ] Obtain remote t0/engine results and cross-vendor review; deliver Ready.

## Handoff

The first new test failed because the old client lacked the generalized text
snapshot interface. After implementation all 22 offline Presto tests passed.
Initial real local Docker execution is being verified. Final SHA-bound evidence
belongs in PR comments/Actions; no evidence-only tracked commit. Owner merges.
