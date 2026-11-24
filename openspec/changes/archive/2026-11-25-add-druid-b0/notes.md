# Notes

## 2026-11-25: investigation, design and implementation

The owner asked for the next change after #95 and #96 merged: route item G3,
Druid 26.0. Implemented with Claude Code (Anthropic).

Established first (design.md has the table): Druid 26.0.0 runs in one
container; Cube Core's Druid SQL equals the kernel's for every case the
kernel compiles; joins run; multi-fact SQL and ordered scans do not.

Done:
- `templates/druid.yaml`, `DialectId::Druid`, `--data-source name=druid`.
- Declared refusals as dialect data; six for Druid, each by name.
- The query context `useApproximateCountDistinct=false` on every Druid result.
- The Druid manifest: 52 of the 85 shared cases, fixtures as SQL ingestions
  generated from the Presto fixtures, a rewritten model, a support list.
- B0-DRUID-001 and -002 transferred from the seed and reworded; the seed keeps
  B0-DRUID-003 and -004.

Evidence, on the working tree that became the commit:
- By hand on local Docker, `apache/druid:26.0.0`, SQL-compatible NULL handling
  and six merge buffers: the generated manifest, fixtures included, parameters
  bound through the SQL API, the declared context applied: 52 of 52 cases
  returned the expected columns and rows.
- The same cases with Druid 26's default NULL handling: one differs,
  `uw_renewal_share_by_branch` (a count filtered on `IS NOT NULL` of a number
  counts every row). With the default two merge buffers
  `joined_many_to_one_counts` fails with "Query needs 3 merge buffers".
- Cube Core 1.8.28, Druid dialect, all 85 cases: byte-identical SQL and equal
  parameters for every case the kernel compiles. Of the 33 the kernel refuses
  on Druid, Cube compiles all but three; the multi-fact and ungrouped ones
  were run and Druid rejected them, the rest await templates (G4).
- Not established: Druid in the checks; no tier exists yet. The engine tiers
  for Presto and Doris were not run locally. Cross-model review: not run.
- `check ci` and the remote checks are recorded on the pull request by
  `scripts/dev deliver`.

Open:
- The harness change: a Druid tier (one container, SQL ingestion, the
  segment-version wait) and a Cube Core comparison with its Druid dialect.
- Which NULL handling and how many merge buffers production's Druid has.
- G4: B1 on Druid. Totals of several facts without a dimension run on Druid
  and are refused with the rest of multi-stage.

Learned: a probe has its own defects. Two wrong results were the probe's: a
load wait that counted rows, and inline data escaped once too few. Both
would have been blamed on the dialect without rereading the probe. The
seeded assumption that Druid cannot join was wrong for 26.0.

- The first checkpoint was blocked by the service test that expected `druid` to be an unimplemented dialect; the service now treats it like the others, and the test asserts a query failure for a data source Druid does not serve.
