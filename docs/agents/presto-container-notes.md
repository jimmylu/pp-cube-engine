# Presto container delivery handoff

PR #42, branch `harness/presto-container`. Product, vendor, golden snapshots and
Cargo dependencies are unchanged. Design and usage are in engines.md. The owner
requested this next harness slice after merging #41; the implementation is not yet
accepted; delivery must obtain passing real-engine evidence before Ready.

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

## Resumed repair

Owner authorized resuming. Run 44991829514 isolated failure to readiness-version;
run 44991970425 reported the exact runtime build `0.293-2f48ff6`. The old comparison
with `0.293` rejected this pinned image. A regression using the real build string
failed before the fix. The runner now requires the complete pinned build identity;
wrong-version regressions confirm fixture setup is skipped and cleanup still runs.
Discovery documentation includes engines-container. Fixed stage and allowlisted
failure messages preserve public-safe diagnostics.

Remaining: obtain a passing real engine run, run final CI, obtain the cross-vendor
report and deliver through Ready. Evidence belongs in PR comments or CI artifacts,
not an evidence-only commit. Owner approval and merge remain required.
