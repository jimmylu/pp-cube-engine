# Design

## Context

Base main e6836b8 includes merged PR #29. Existing B0 golden files live in
aggregation, filters, joins, time and security integration test binaries.
Their helpers already hold complete typed inputs; Compiled derives Debug and
PartialEq and contains only ordered fields. A separate result.rs test compares
one fingerprint between processes, which does not cover the golden contract.

## Decisions

Use shared test support to replace each suite's compile import with a checked
wrapper. On success, spawn that same integration-test binary four times, running
only support::worker. Send a JSON envelope on stdin containing complete typed
model/query/timezone/security filters; reconstruct those typed inputs in the
worker and call the production compile directly. Explicitly serialize Presto
selection, the only supported dialect, and preserve request_id as well. Existing
B0-DET-002 tests separately verify correlation exclusion. Workers never call the wrapper, avoiding recursion.

Compare complete derived Debug representations, including SQL, parameters,
columns, used cubes and fingerprint. Debug is a test transport/comparison here,
not a fingerprint encoding contract or public wire API. A tagged single-line
record separates result data from libtest output; require successful child exit
and exactly one result. Compare each of four children to the successful parent.
Reject empty/malformed/missing records rather than letting empty output pass.

All current golden suites use this import, so existing and future successful
cases in their shared helpers automatically gain the check. Keep all existing
snapshots unchanged. Error cases return normally without launching workers.
A small test checks that changing each full-result field changes the comparison
record, guarding against accidentally reducing coverage to SQL or fingerprint.

Alternatives: hand-maintaining another fixture catalogue duplicates golden inputs
and risks drift; spawning whole suites recursively causes recursion or requires
additional harness configuration. This solution stays wholly within compile tests.

## Risks / Validation

Per-case process overhead is deliberate; four independent process seeds are
required. Child failures report status/stderr and case input context. Concurrent
parent tests exchange inputs through separate pipes, not global files or shared
mutable environment. Model/query JSON roundtrip must preserve the original typed
inputs; existing fingerprints and full results check that assumption as well.
Compile remains free of process/IO code. Run all five golden suites, strict specs,
fast and CI. Any observed upstream nondeterminism is a spec contradiction: record
it and stop rather than sorting output, changing goldens or patching vendor.
Archive before final review as docs/agents/workflow.md requires. Evidence belongs
in PR comments/artifacts; owner approves and merges.
