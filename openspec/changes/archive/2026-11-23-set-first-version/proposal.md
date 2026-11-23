# First version

## Why

The workspace has carried the placeholder version 0.0.0. The owner deferred a
real version until the kernel had been compared with Cube Core; that
comparison now runs on every engine-relevant pull request, so responses and
fingerprints should name a real version.

## What Changes

- The workspace version becomes 0.1.0. It is the kernel version every response
  reports and every fingerprint covers.
- **Fingerprints change** for every compilation, because the kernel version is
  part of them. A caller's plan cache keyed on fingerprints starts empty, which
  is what the fingerprint is for.
- `CHANGELOG.md` records what 0.1.0 contains and what it does not.
- No specification changes: the determinism spec already says the fingerprint
  covers the kernel version. No SQL, golden or contract change.

Non-goals: tagging, publishing an image or a release, a release workflow (the
owner decided images are built and verified in CI only), any promise of
compatibility before 1.0.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. This change sets `skip_specs: true`.

## Impact

Paths: `Cargo.toml`, `Cargo.lock` (the six workspace members), `CHANGELOG.md`,
and this change folder.
