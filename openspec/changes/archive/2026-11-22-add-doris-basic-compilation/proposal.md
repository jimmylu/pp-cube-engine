## Why
Only Presto currently emits SQL. Add the first bounded Doris compilation path while retaining full B0 matrix and engine acceptance in the seed backlog.
## What Changes
Add Doris templates and dialect identity, expose it through Compile and gRPC, and support basic single-cube aggregation/grouping with scalar comparisons and sorting/pagination. No Presto SQL changes.
## Capabilities
### New Capabilities
- `dialect-doris`: initial basic compilation, not the full seed matrix.
## Impact
Scope: templates/doris.yaml, crates/pp-cube-dialect/, crates/pp-cube-compile/, crates/pp-cube-service/src/grpc.rs, crates/pp-cube-service/tests/grpc.rs, this change and openspec/specs/dialect-doris/spec.md. No harness/vendor edits. No joins, time, patterns, real Doris execution or T2 acceptance in this PR.
