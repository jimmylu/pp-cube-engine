# Design

Use the existing pinned planner and adapter time semantics. Add Doris templates
for UTC DATETIMEV2(3) bounds, UTC-to-query-zone conversion and calendar truncation.
Remove the time-dimension rejection and filter allowlist (the only formerly rejected Operator variants are the six B0 date operators); offset without limit remains unsupported.
Reusing MySQL DATE_FORMAT would complicate quarters/weeks; local Rust bucket
calculation would duplicate planner logic. Doris DATE_TRUNC directly supports B0.

Validate goldens for all granularities in Tokyo and all date operators. An opt-in
integration test compiles live requests and passes SQL/parameters to a Python
fixture verifier on the existing disposable Doris harness. This test lives with
product tests and does not alter harness paths. Assert UTC boundary exclusion,
Tokyo buckets, DST and security OR conjunction. The kernel remains compile-only.
Doris documentation: https://doris.apache.org/docs/4.x/sql-manual/sql-functions/scalar-functions/date-time-functions/date-trunc/
https://doris.apache.org/docs/4.x/sql-manual/sql-functions/scalar-functions/date-time-functions/convert-tz/
