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

## Review clarification

At published head 20dbf1ee662b3676af5c5a09aa2e0cd789435984 the explicit tier ran:
`cargo test -p pp-cube-compile --test doris_time real_doris_temporal_execution -- --ignored --nocapture`.
Result: `Doris live-compiled temporal execution: PASSED — 13 cases`;
`test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out`.
Evidence: https://github.com/jimmylu/pp-cube-engine/pull/58#issuecomment-7080782960.
This is an executed test, not traceability-only coverage. T0 intentionally ignores
Docker-dependent tests. The seed design requires an execution tier to be built
and passed before archive; this explicit tier is built and passed. It does not
require every T2 scenario to run in T0 or scripts/dev. Future Doris PRs must run
both the standard Docker smoke and this temporal integration command before Ready.
Adding automatic invocation to the harness is a separate harness-only change.
Review should distinguish the existing smoke's limited coverage from this new
live-compiled execution suite. The scenario is explicitly labelled (T2).
