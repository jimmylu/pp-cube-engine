# Session notes

## Planning history

PR #27 merged; branch starts from origin/main 8e85004. User requested the next PR.
Planning uses OpenSpec propose; implementation has not begun. All seven tasks
remain pending. No product code, dependencies, main specs, snapshots or vendor
files changed in this planning increment.

Selected complete compile result plus fingerprint identity, transferring the
whole requirements and preserving B0-RESULT-001/B0-DET-002. Classified errors and
all-golden process determinism remain in the seed. Adapter currently returns SQL
and parameters only. TopLevelPlanner::plan does not directly return projection
metadata. The first task must prove the usable pinned projection/source API;
stop for decision if faithful metadata needs vendor changes or a planner fork.

The current typed Model has no external bundle version/digest. Design defines its
identity by canonical full typed content; SQL/metadata remain covered. Request id
will be explicit correlation-only context. No new function or SQL behavior is
promised, and golden output changes are not authorized.

Next: review the concrete plan, then apply the tasks in this same Draft PR.
Archive is after implementation and before final review, per repository workflow.

Planning verification: strict OpenSpec validation passed seven items. Full CI
passed 15 steps, 0 skipped on the planning working tree. No implementation or
T1/T2 execution is claimed. Exact-head evidence will be posted on the Draft PR.

## Apply feasibility investigation

Final physical projection is local to TopLevelPlanner::plan. Public resolved
QueryProperties::all_members(false) exposes stable member ids/aliases; the final
SQL AST provides actual projection order. Pair these, failing on missing or
ambiguous identity rather than guessing. LogicalSchema::all_dimensions and the
physical query processor confirm dimensions/time buckets precede measures.
Lineage can use rendered table factors, not join-graph candidates. This pairing
uses final SQL plus resolved planner symbols and does not fork the planner.


## Implementation

Implementation authorized by the user. Tasks 1–5 are complete. Projection aliases
are parsed from final rendered SQL and matched to resolved upstream symbols;
time buckets use their base member identity and explicit granularity. Lineage is
selected by rendered table factors and cube aliases, then physical table identifier
components are decoded through the parser. Ambiguous metadata fails internally.

The facade adds columns, used_cubes, fingerprint and correlation-only request_id.
Fingerprint v1 uses canonical recursively sorted JSON (ordered arrays retained),
SHA-256 of the complete typed model, and a framed JSON record of query, dialect,
timezone, security filters, package version and full pin. No runtime Git/IO enters
compilation; the pin is a constant guarded by a repository-gitlink test. Upstream
upgrades must update it with the pin. Existing SQL/parameter snapshots pass.

Tests cover public parameter/column correspondence, time buckets, join intermediate
sources, quoted cube SQL, sql_alias, hidden security/time filters, repeated members,
request-id exclusion, model/query/context sensitivity, build identity, canonical
numbers/object ordering and fresh-process comparison. The all-golden process suite
remains proposed. Only Presto is currently representable as a dialect; the exhaustive
dialect match ensures a future variant requires an identity choice.

Dependency review: sqlparser reuses =0.64.0 with std/visitor, default stack extension
disabled. New SHA-256 uses sha2 0.11.1; cargo deny passed advisories, bans, licences
and sources. Cargo.lock changes require risk:upstream; upstream pin unchanged.
Scope exceeds 400 lines including tests/dependencies, retained as one complete
successful-result behavior. No vendor/harness or existing golden snapshots changed.
Final working-tree CI passed 15 steps, 0 skipped (55s); fast also passed before
checkpoint. Implementation is published on PR #28. Both implemented requirements
are synced into main specs, and this folder is archived before final review per
repository workflow. Cross-vendor review and exact-head CI evidence remain final
delivery work, recorded on the PR without changing the reviewed commit. Owner
approval/merge follows. T1/T2 not run.

Current implementation tasks: 7/7 complete. Archive is complete; final delivery review is pending.

Cross-vendor review of a83420b identified two B0-RESULT-001 test gaps. The
public correspondence case now groups sales by region and asserts ordered
parameter values plus exact alias/member/semantic-type correspondence. No
production code change was needed; final delivery is rerun after this test fix.

Second cross-vendor review requested an explicit repeated-selection count and
defensive CTE source handling. The pinned planner preserves two selected measure
occurrences, now asserted exactly. Metadata rejects CTE/derived cube sources even
for direct adapter callers; public compile already rejects them through model
validation. Added public/direct-adapter regression coverage and rerun delivery.
