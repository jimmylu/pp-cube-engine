# Proposal

## Why

The facade returns SQL and ordered parameters only. Callers cannot identify
output members, inspect actual source usage or key a cache by compilation inputs.
Complete the successful-result contract before expanding dialects or service APIs.

## What Changes

- Add ordered output columns (alias/member/type), used cubes and physical tables.
- Add a stable 32-byte fingerprint covering model identity, query, semantic
  context, kernel version and upstream pin; exclude the request id.
- Transfer the complete compile-result and fingerprint-identity requirements
  from the seed, preserving B0-RESULT-001 and B0-DET-002.

## Capabilities

### New Capabilities

- compile-result: complete successful compilation results.
- determinism: fingerprint identity; independent-process completeness remains proposed.

### Modified Capabilities

None.

## Impact

Scope: adapter src/tests, compile src/tests/fixtures, model src/tests only if a
shared source-lineage helper is needed, their Cargo manifests, Cargo.lock,
this change, main compile-result/determinism specs at archive, and the two seed
deltas for complete requirement transfer. Preserve existing successful SQL and
parameters. No golden SQL changes are authorized. New result fields and a request
id field affect struct-literal callers. Stay at B0; vendor and harness are read-only.

## Non-goals

Classified errors/unsupported-query parsing, new dialects, services, SQL execution,
credentials, caching, upstream changes and B0-DET-001's all-golden four-process suite.
The seed retains its classified-error and repeatable-result requirements.
