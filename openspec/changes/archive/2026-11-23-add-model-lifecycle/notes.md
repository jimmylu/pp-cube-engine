# Notes

## 2026-11-23: design and implementation

The owner asked to continue with the recommended plan after #73 merged. Second
unit of the productization route. Implemented with Claude Code (Anthropic).

The open design question was how model versions arrive at run time. Chosen:
explicit gRPC operations rather than a watched directory; design.md gives the
reasons. **For the owner to confirm at review**, together with: versions are
immutable and republishing equal content is a no-op; published versions are in
memory only.

Done:
- Catalog: ordered listing and retirement (B0-MODEL-009).
- Content digest `sha256:<hex>` of the typed model, computed by the service;
  the caller-supplied digest is gone. ValidateModel returns it (B0-API-014).
- `CubeService::publish`, `retire` and `versions` through `&self`; idempotent
  publish, refusal of different content, retirement that does not break running
  compilations (B0-API-013, including a concurrent test).
- gRPC ListModels, PublishModel and RetireModel, refused unless the host enabled
  management (B0-API-015). Executable: repeatable `--model version=path`,
  `--allow-publish`; `--version` and `--digest` are removed.

Evidence: `check ci: PASSED — 15 steps, 0 skipped, 244s @ c435730+dirty` on the
working tree that became the implementation commit. A first run failed the
format step; `cargo fmt` was applied and nothing else changed. No golden
changed. Remote checks are recorded on the pull request. Cross-model review:
not run.

Open: management has no authentication (next unit: deployable and operable);
published versions do not survive a restart; no limit on the number of versions;
the planner context is still rebuilt per compile (performance unit).

Learned: `ModelBundle::new` validates again inside `publish`, so a publish
validates the typed model twice (once for located issues from text in the gRPC
layer, once in the core). Cheap today; worth folding when the cache arrives.
