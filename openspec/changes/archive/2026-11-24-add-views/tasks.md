# Tasks

## 1. Establish

- [x] 1.1 Put queries on a view to Cube Core 1.8.28 and to the kernel; record the table in design.md. Verified by comparing SQL and parameters of 18 queries.
- [x] 1.2 Read how the planner's fixtures present a view to the bridge; record the form in design.md.

## 2. Contract

- [x] 2.1 `views` in the model, join-path references, `Model::expanded` and `Model::underlying`; test B1-MODEL-003; update the reference-form test.
- [x] 2.2 Validate views; test B1-VALID-004.

## 3. Compile

- [x] 3.1 Flag view cubes and members for the planner, compile join-path references, expand when a model is prepared; test B1-VIEW-001 and -002 with goldens.
- [x] 3.2 Translate view members in the join, window and count-filter checks; test B1-VIEW-003.
- [x] 3.3 Doris goldens (B1-DORIS-004).
- [x] 3.4 Service: compile on views, list them in GetMeta; test B1-API-001.

## 4. Evidence

- [x] 4.1 Add a view and five execution cases for both dialects; verify existing cases' SQL is unchanged.
- [x] 4.2 Run `conformance` and `doris-container` locally; record the lines in notes.md.
- [x] 4.3 Update `README.md`, `docs/guide.md`, `CHANGELOG.md`; verify `openspec validate --all --strict` and traceability.
- [x] 4.4 Run `scripts/check.sh ci`; record the last line in notes.md.

## Workflow follow-up

- Sync and archive the deltas before final review, in the same Draft PR.
- With this the eight units of the route are done; the owner decides what follows (dialects were next).
