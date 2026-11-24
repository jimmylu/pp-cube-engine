# Design

## Context

See proposal.md. The kernel does no IO; the service does. The service already
publishes, lists and retires immutable versions in memory with a verified
content digest.

## The layout

```text
<root>/
  property/                      business line, level 1
    motor/                       level 2 (optional)
      fleet/                     level 3 (optional)
        models/                  asset type: the one this service reads
          v12/                   version
            manifest.json        {"schema": 1, "digest": "sha256:…"}; written last
            cubes/policies.yml   one cube, named after the file
            views/underwriting.yml
        metrics/…                another asset type: left alone
```

## Decisions

- **The asset type is a reserved directory name between the business line and
  the version.** Business lines are found as the one to three path parts
  before `models`. A new asset type is a new name at the same place, and this
  service does not look at it. Alternative: asset type first
  (`models/<line>/…`); rejected because the owner asked to isolate by business
  line first, and access to a line's subtree is then one rule in NAS and S3.
- **The version a caller names is `<line>/<version>`**, such as
  `property/motor/v12`. The wire contract's `model_version` is a string and
  needs no change; ListModels and identities show the same name. Alternative:
  a separate business-line field; deferred until a caller needs to filter.
- **Model files follow Cube's recommended project structure:** `cubes/` and
  `views/`, one definition per file, the file named after it in snake_case.
  The kernel enforces what Cube recommends, so a directory that loads here is
  also laid out the way Cube's tooling expects. Subfolders inside `cubes/`
  are not read; business lines are the grouping.
- **Assembly is pure and lives with the contract** (`pp-cube-model::assemble`),
  so the WebAssembly validator can use it later. Files are ordered by folder
  and name before they are joined, so the digest does not depend on the order
  a store lists them in.
- **`manifest.json` marks completeness.** A directory tree can be renamed into
  place atomically; an S3 bucket cannot. A marker written last works on both.
  It carries a schema number, for room to grow, and optionally the digest the
  publisher got from ValidateModel, which makes a torn or altered publication
  a rejection instead of a different model.
- **Versions are immutable and read once.** A loaded version is not read
  again, so a pass costs one listing. Changing a version's files in place has
  no effect until a restart, and then is caught by the digest when the
  manifest has one. Publish a new version instead.
- **The store is the truth for its versions.** Removing the manifest retires;
  a store version retired through the service comes back at the next pass;
  versions from `--model` or PublishModel are never retired by the store.
- **A bad publication is not an outage.** The first pass is part of starting,
  and a store that cannot be listed stops the process, like a missing model
  file. A version that cannot be offered is logged and counted and the others
  are served. Later failed passes change nothing.
- **A store is `list` and `read`.** That is what a directory tree and
  ListObjectsV2 with GetObject have in common; the layout is parsed from keys
  by a pure function with its own tests.
- **Polling.** Replicas read independently, so a version is everywhere one
  interval after its manifest. Sufficient for a publisher that publishes and
  then announces; noted in the service guide.

## Risks / Trade-offs

- Eventual availability across replicas → documented; the semantic layer waits
  an interval, or callers retry an unknown version.
- Issue paths of a rejected model point into the assembled model, not into a
  file → the reason and the first issue are reported; file positions come with
  route item G12.
- Listing a very large store every interval → bounded at 100000 keys and seven
  levels; a prefix per business line is the remedy if it is ever needed.
- The owner asked for NAS and S3 together; this delivers the layout, the
  trait and NAS. S3 follows as its own change.

## Migration Plan

Additive. `--model` and PublishModel work as before. Evidence:
`scripts/check.sh ci`, remote `t0`, `image`, `aarch64` and the engine checks.
