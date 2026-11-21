# Tasks

Each task is bounded to two hours; stop and record unresolved scope/API decisions.

- [x] 1. Trace pinned final projection/schema and source access APIs in a read-only spike. Prove adapter metadata can correspond to actual SQL without vendor changes; record selected seam and limitations before production edits.
- [x] 2. Add a failing public B0-RESULT-001 parameter/column test and implement OutputColumn/Planned metadata. Cover aliases, semantic types and actual projection order with hidden members, time buckets and repeated selections; preserve existing SQL snapshots.
- [x] 3. Add B0-RESULT-001 lineage tests for join intermediates, implicit count, security dependencies, sql_table and cube sql. Implement actual source tracking and qualified identifier handling; exclude unused cubes and graph search candidates.
- [x] 4. Add canonical model/query/context encoding and SHA-256 with reviewed dependency licence/lockfile scope. Test metadata precision, recursive object-key ordering, ordered arrays and framing; document the typed-model digest contract.
- [x] 5. Add request_id and fingerprint to the public facade. Implement B0-DET-002 sensitivity for every input, repeated identity, request-id exclusion and one fresh-process comparison; preserve SQL and parameter results.
- [x] 6. Run model/adapter/compile tests, strict specs, fast and CI. Review scope, API compatibility, no vendor/harness edits and unchanged goldens; update handoff notes and complete only implemented tasks.
- [x] 7. Sync/archive complete requirements before final review and checkpoint/push the implementation. Apply risk:upstream if Cargo.lock changes.

Final delivery gate (tracked against the exact head in PR comments/artifacts): publish CI evidence, pass cross-vendor review and remote t0, then mark Ready. This gate is pending until the PR records success; the completed implementation tasks above do not assert acceptance of the final head. Owner merges.
