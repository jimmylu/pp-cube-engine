# Presto container delivery handoff

PR #42, branch `harness/presto-container`. Product, vendor, golden snapshots and
Cargo dependencies are unchanged. Design and usage are in engines.md. The owner
requested this next harness slice after merging #41; the implementation is not yet
accepted and must remain Draft.

## Implemented

Pinned Presto 0.293 amd64 container provisioning, memory catalog installation,
ephemeral loopback port, bounded readiness/version probe, existing fixture setup,
existing golden verifier reuse, owned-container cleanup and a separate CI job.
Offline lifecycle regressions cover success, failures and create timeouts. Review
repairs address cleanup on uncertain create outcomes and failure step summaries.

## Evidence and remaining work

- `31155b3`: local CI passed all 15 steps in 83 seconds; remote t0 passed.
- `acfddd4`: local CI passed all 15 steps in 82 seconds; remote t0 passed.
- Real engine runs failed readiness. The controlled diagnostics confirmed a running
  container, no OOM, no server exception classes and a CLI 'not registered' error.
- `10b8034`: local CI passed all 15 steps in 76 seconds. Replaced unsupported `version()` with the official
  `SELECT DISTINCT node_version FROM system.runtime.nodes` query. Fast passed.
  The real engine job now fails quickly with ValueError instead of the readiness
  timeout: run 44991214339, job 137112462936. This does not establish execution
  correctness; no passing T2 evidence exists.
- Earlier Opus review raised two implementation findings, subsequently repaired.
  Its next report requested actual passing engine evidence and verification of the
  pinned image layout. No final accepted cross-vendor report exists for this head.
- The image config was inspected through the public registry: it supplies
  /opt/presto-cli, /opt/presto-server/etc/catalog, Java 11 and a 1 GiB JVM heap.
  The runner has a 2 GiB container limit. Registry inspection is not runtime proof.

Stop after repeated failed engine checks and two probe adjustments under AGENTS.md.
Do not mark Ready, bypass the engine failure, remove the new workflow or weaken
result assertions. Continue only in a resumed repair session. First add a fixed
stage indicator and distinguish the runner's own ValueError messages without
printing endpoint names or raw server responses. Determine whether the failure is
version comparison, HTTP continuation-origin validation, query execution or result
comparison. Then write a regression at that boundary and obtain a real passing
engine run before final delivery. Use the existing supplemental review context:
`presto.py`, `presto-fixture.sql` and the grouped-sum golden snapshot.

This handoff changes documentation only. Its own full CI and cross-vendor review
are not claimed; check and review evidence must bind to the eventual final SHA.
Owner approval and merge remain required.

## Resumed repair

Owner authorized resuming the bounded repair. Fixed stage and allowlisted local
error categories now distinguish readiness, fixture and golden failures without
printing server messages. Discovery documentation includes engines-container.
Real engine validation and final cross-vendor delivery remain pending.
