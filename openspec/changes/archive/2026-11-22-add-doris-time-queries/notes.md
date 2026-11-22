# Notes

Using change add-doris-time-queries. Owner authorized next design and development.
B0 only; real local Doris Docker is mandatory. No owner decision is open.

Implemented Doris DATE_TRUNC, CONVERT_TZ and DATETIMEV2(3) UTC-bound casts.
The native prepared statement path executed 13 freshly compiled requests on real
local Docker Doris 4.1.3, including Tokyo millisecond bounds and New York DST.
All passed; the owned container was removed. Eleven new SQL goldens reviewed;
no previous golden changed. cargo insta review reports no pending snapshots.
Remaining: final checks, publication and cross-vendor delivery. No blocker.
Exact-head evidence belongs in PR comments, not a follow-up evidence commit.

## Final review resolution

Formal B0-DORIS-010 T2 acceptance remains proposed in the active seed Doris delta.
Only B0-DORIS-009 compilation and the revised B0-DORIS-004 boundary are archived.
The opt-in live-compiled test passed on local Docker at both published heads
20dbf1e and b3a5ee9. Latest evidence:
https://github.com/jimmylu/pp-cube-engine/pull/58#issuecomment-7080904561.
The existing standard Docker tier does not invoke temporal cases. A separate
harness change must place/integrate that execution coverage before formal T2
acceptance. No harness files are changed here. Product behavior is complete;
formal tier integration is deferred explicitly, not claimed as complete.
Final SHA-bound checks and review evidence are recorded in PR comments.

Execution-tier integration is tracked in https://github.com/jimmylu/pp-cube-engine/issues/59.
