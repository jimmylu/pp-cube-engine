# Tasks

## 1. Establish

- [x] 1.1 Run Druid 26.0.0 in one container; load rows by SQL ingestion; record what it accepts in design.md.
- [x] 1.2 Ask Cube Core's Druid dialect for the SQL of all shared cases; compare with the kernel's.

## 2. Dialect

- [x] 2.1 `templates/druid.yaml` and `DialectId::Druid`; test B0-DRUID-001.
- [x] 2.2 Declared refusals in the dialect's data, checked by the adapter; test B0-DRUID-002.
- [x] 2.3 A session property for every statement; test B0-DRUID-005.

## 3. Evidence

- [x] 3.1 The Druid manifest, its fixtures, the rewritten model and the support list; test B0-EXEC-006 with goldens.
- [x] 3.2 Run the manifest by hand on Druid 26.0.0; record it in notes.md.
- [x] 3.3 Update the guide, README, changelog and the seed change.

## Workflow follow-up

- Archived in the same commit as the implementation.
- A harness change: a Druid execution tier and a Cube Core comparison for Druid (B0-DRUID-003, B0-DRUID-004).
- G4: B1 on Druid.
