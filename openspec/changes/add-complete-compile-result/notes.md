# Session notes

## Current state: next PR planning

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
