# Doris empty page case

## Why

The Doris tier's stored-golden assertions, about to be removed, include one that
no execution case repeats: a page whose offset lies past the last group returns
no rows. Review of the tier cleanup (#67) found the gap.

## What Changes

- One Doris execution case, `page_past_last_group`: the four groups of the
  existing pagination case with `limit 2, offset 4`, expecting no rows.
- Its block joins the reviewed Doris golden; the manifest grows from thirty-nine
  to forty cases.
- New scenario B0-DORIS-011 under the Doris B0 acceptance requirement.
- Baseline level: B0. No template, generated SQL or existing golden block changes.

Non-goals: harness paths (#67 removes the stored-golden assertions after this
merges); Presto; offset without limit, which stays unsupported on Doris.

## Capabilities

### New Capabilities

None.

### Modified Capabilities
- `dialect-doris`: the Doris B0 acceptance requirement gains B0-DORIS-011.

## Impact

Paths: `crates/pp-cube-conformance/src/doris.rs`, its `tests/manifest.rs` and
snapshot, `openspec/specs/dialect-doris` and this change folder. Observable
output: one more manifest case and golden block.
