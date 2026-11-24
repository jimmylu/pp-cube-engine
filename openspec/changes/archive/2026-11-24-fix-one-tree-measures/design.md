# Design

## Context

See proposal.md. `Joins::check` groups what a query reads into facts and
asked each for a join tree of its own to the shared dimensions.

## Established against Cube Core 1.8.28

Thirteen queries on the scenario model, before and after the change.

| Query | Cube Core | Before | After |
| --- | --- | --- | --- |
| claim count and outstanding by line; with filters, order and a limit; by loss month; as totals | one SELECT from `claims` | query error | same SQL |
| the average-outstanding formula by line | one SELECT | same SQL | same SQL |
| paid and outstanding by line; with written premium by region | CTEs from `claim_payments`; the reserve over distinct claim keys | query error | same SQL |
| outstanding alone by line; outstanding with earned premium by line | error: no join path | query error | query error |
| outstanding by a dimension of `claims` | one SELECT | same SQL | same SQL |
| a count filter beside the reserve | compiles | query error | unsupported (#83) |

## Decisions

- **Ask for the whole tree first.** With more than one fact, the check builds
  the tree of all cubes the facts and the dimensions read. If it exists, a
  fact without a tree of its own is accepted; if not, the fact is reported as
  before. Nothing is added for a query of one fact, whose own tree is the
  whole tree. Alternative: treat a cube joined one_to_one as part of the fact
  that joins it; rejected because the planner decides that, and the rule
  would have to mirror it.
- **The count-filter rule is still applied on the whole tree**, as before.
- **The two scenario queries become execution cases.** Paid and outstanding
  by line shows a property of Cube's SQL worth pinning: counted from
  payments, a reserve belongs to a claim that was paid on, so an open claim
  without a payment contributes nothing.

## Risks / Trade-offs

- A wider acceptance could let through a query the planner then cannot join →
  the planner asks the bridge for every tree it uses, and a missing one is
  still reported; thirteen probes and two engines show none.

## Migration Plan

Additive. Revert to restore the refusal. Evidence: `scripts/check.sh ci`,
remote `t0`, `presto-smoke`, `doris-smoke`, `cube-core`.
