# Design

## Context

See proposal.md. The pinned planner supports views through the same bridge
as cubes: a cube flagged as a view, whose members' expressions are references
with a join path.

## What Cube Core 1.8.28 does, established first

Eighteen queries on a view over four cubes were put to Cube Core on Presto
and to the kernel. Every query both compile has byte-identical SQL and equal
parameters (but for Cube's default row limit).

| Query on a view | Cube Core on Presto | Here |
| --- | --- | --- |
| measures by own, prefixed and two-hop dimensions; filters, time, order, page | the cubes' SQL with the view's column aliases | same SQL |
| sum over rows the view's join repeats | CTEs; inner aliases are the cubes', outer the view's | same SQL |
| rolling, shifted, declared granularity, formula, filtered, distinct, ungrouped | as on the cubes | same SQL |
| a root measure beside a one_to_one measure | one CTE per cube, where the cubes' own query is flat | same SQL |
| view member beside a cube member | compiles | same SQL |
| member the view does not take | error | query error |
| filter on a count over repeated rows | compiles | unsupported (as on the cubes) |

## Decisions

- **Expand views to cubes, and let the planner plan.** `Model::expanded` adds
  one cube per view: each dimension and measure refers to the member it takes
  as `{join.path.member}`, measures as number formulas, which is how the
  planner's own fixtures and Cube's compiler present a view. The bridge flags
  the cube as a view, its members as included, and passes the join paths of
  more than one cube. Alternative: rewrite a query on a view into a query on
  cubes and rename the columns afterwards; rejected because Cube's SQL names
  inner columns after the cubes and outer ones after the view, which a rename
  cannot reproduce, and because the planner does differ for views (the
  one_to_one row above).
- **Expansion happens once, when a model is prepared.** The content digest is
  of the model as written; the prepared model holds the expansion. `compile`
  on an unprepared model prepares it, as before.
- **A new reference form, internal.** `{a.b.member}` parses as a join-path
  reference so that expanded views resolve like any expression. Validation of
  a model as written reports it: the contract's three forms are unchanged.
- **The kernel's checks see the members behind a view.** The join check, the
  window and shift checks and the count-filter refusal translate a view
  member to the member it stands for first (`Model::underlying`), so no rule
  is bypassed by going through a view. Messages name the member behind the
  view.
- **One join path, as before.** A view's join path must be a chain of joins
  the model declares, and all paths of a view start at one cube, so its
  members always form one tree. The kernel still refuses a model in which two
  paths connect the cubes of a query; a view's path does not choose between
  them. Cube would use it as a hint.
- **Narrow views.** Dimensions and measures only; no members declared on the
  view, no segments, no `alias`. Each is a model error at its field.
- **GetMeta lists views as cubes**, after the cubes, with the measure type of
  the member behind. The wire contract is unchanged, so a client cannot yet
  tell a view from a cube other than by its lack of joins.

## Risks / Trade-offs

- A query through a view is not always the cubes' SQL (the one_to_one row) →
  it is Cube's SQL, pinned by an execution case that runs against Cube Core.
- `includes: "*"` leaves segments out, where Cube takes them → documented;
  taking segments needs segment references, which the contract does not have.
- No view flag in GetMeta → an open point for the owner; it needs a wire field.
- Agreement is established where checked: 18 probes and 70 execution cases.

## Migration Plan

Additive. Revert to restore the refusal. Evidence: `scripts/check.sh ci`,
remote `t0`, `presto-smoke`, `doris-smoke`, `cube-core`.
