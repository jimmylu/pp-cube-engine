# Doris B0 acceptance

## Why

Doris compiles the B0 matrix, but several members have no Doris golden and most
have never run on a real engine. An audit compiled and executed a case for every
matrix member on Doris 4.1.3: all pass with the current templates, so the gap is
evidence, not behavior. B0-DORIS-001 and B0-DORIS-002 can leave the backlog.

## What Changes

- Add twenty-six Doris execution cases on one retail fixture, covering every
  B0 matrix member beside the thirteen temporal cases: six aggregate types,
  filtered measure, ratio, segment, equals with several values, notEquals,
  boolean equals, set, notSet, four number comparisons, six pattern operators,
  measure filters, nested and/or groups, two-member order with limit and offset,
  a plain absolute range, query timezone precedence, a row-filtered cube source,
  a joined dimension, a security-only two-hop join and a security pattern beside
  a user OR.
- Add one reviewed golden holding the SQL and ordered parameters of every Doris
  execution case, so each executed case is also a golden case.
- Transfer the complete Doris B0 compilation and Doris B0 acceptance
  requirements, with B0-DORIS-001 and B0-DORIS-002, from `seed-baseline-specs`
  to `dialect-doris`. The seed Doris delta loses its last requirement and is
  deleted.
- Baseline level: B0. No template, generated SQL or existing golden changes; the
  new golden is new output of new cases.

Non-goals: any harness path (the Doris tier already runs whatever the manifest
holds; removing its stored-golden assertions is a later harness pull request);
Presto, Druid and Cube Core comparison; offset without limit, which Doris keeps
declaring unsupported (B0-DORIS-004); T3 on real clusters.

## Capabilities

### New Capabilities

None.

### Modified Capabilities
- `dialect-doris`: adds Doris B0 compilation (B0-DORIS-001) and Doris B0 acceptance (B0-DORIS-002).
- `execution-cases`: B0-EXEC-001 covers every declared Doris case, not only the thirteen temporal ones; its title is kept because a scenario title is its identity.

## Impact

Paths: `crates/pp-cube-conformance/` (model fixture, cases, test, new snapshot,
`insta` as a dev-dependency at the workspace's existing pin), `Cargo.lock` for
that edge only, `openspec/changes/seed-baseline-specs/` (moved requirements and
notes), `openspec/specs/dialect-doris`, `openspec/specs/execution-cases` and
this change folder. Observable output: the Doris manifest grows from thirteen
to thirty-nine cases; compile results are unchanged.
