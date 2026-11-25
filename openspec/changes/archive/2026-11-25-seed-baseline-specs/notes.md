# Session notes

## 2026-11-22: boundary proposal transfer

The complete Checks before compilation and B0 boundary requirements, including
B0-QUERY-009 and B0-AGG-003, move to add-query-boundary-classification.
The emptied contract-query and compile-b0 delta files are deleted. This is
planning only, not implemented behavior or execution acceptance. The new
change's review.md records the overall gaps and recommended delivery sequence.

## 2026-11-19: step 5 proposal preparation

Owner decision: approved in conversation on 2026-11-19, resolving
https://github.com/jimmylu/pp-cube-engine/issues/5. Finish step 5 with configuration,
public design and a baseline proposal; archive capabilities only after their
implementation and meaningful tests. Do not weaken the traceability gate.

Prepared by Codex (OpenAI GPT-6):
- Confirmed OpenSpec 2.1.0, project-scoped skills and repository root.
- Added English project context and artifact rules to config.yaml.
- Compared the public design with source chapters 1–8 and appendix A; retained
  neutral examples and clarified that measurements are historical, not rerun.
- Split public design into contract (207 lines) and kernel (362 lines).
- Drafted twelve capability deltas with 31 unique scenario ids, plus proposal,
  design and tasks. No product code, main specs or harness files were changed.

Publication order (separate PRs):
1. Pending OpenSpec harness prerequisite on harness/openspec. GitHub lists the
   earlier guard PR #4 as merged, but no OpenSpec harness PR exists.
2. Public contract document (207 lines).
3. Kernel document and config (388 lines combined).
4. This change folder only, on change/seed-baseline-specs (below 400 lines).
The current local branch holds all uncommitted preparation files. Do not open
one combined PR or include inherited harness commits in the proposal PR.

Verification:
- Strict OpenSpec validation: 1 change passed, 0 failed.
- Structural audit: 12 capabilities, 31 unique B0 ids, no main specs.
- CI first stopped on the sandbox advisory database lock; an authorized rerun
  reached typos and found an old commit hash misread as a word in these notes.
  Removed that obsolete historical detail; the rerun passed.
- CI uses BASE_REF=c1faba9, the existing OpenSpec harness prerequisite, to check
  this documentation increment independently. This is not evidence for a PR
  against current main. Re-run default CI on each eventual PR branch.
- Product tests currently number zero; no dialect/conformance execution claimed.

Command: `BASE_REF=c1faba9 scripts/check.sh ci`.
Summary: `check ci: PASSED — 14 steps, 0 skipped, 20s @ c1faba9`.
This run included the untracked preparation files. The script's commit suffix
does not identify their contents; it must not be used as committed-PR evidence.

No commit, push, merge or archive has been performed. The seed is a proposal
backlog; transfer complete requirements to implementation changes as described
in design.md before archiving them.

## 2026-11-19: review and revision

Reviewed, then revised at the owner's direction, by Claude (Anthropic) through
omp. The reviser is now also an author, so task 2.4 needs a reviewer from
another vendor.

Review of the first draft, findings resolved in this revision:
- Scenarios no T0 Rust test can cover (upstream integrity and upgrade review,
  engine execution). `upstream-pin` now holds one kernel-observable scenario;
  each dialect has a T0 golden scenario and separate `(T1)`/`(T2)` scenarios;
  design.md says when those may be archived.
- Fingerprint covered the request id; it is now excluded.
- Security filters had no fail-closed rule; added B0-SEC-003.
- Specs cited draft design section numbers; lists are now inline.
- Druid joins were undefined; they now return unsupported (B0-DRUID-002).
- `service-api` relied on a load operation the design lacks; reworded to
  "available" versions, mechanism left open.
- Added B0-JOIN-003 (data sources) and B0-VALID-004 (expression constraints);
  rejections now name their error class. The attempted rename of B0-AGG-003
  was reverted in task 2.4: harness examples do not justify changing an id.
- design.md: transfer scope, scenario splitting, folder deletion, tiers.

Learned: OpenSpec 2.1.0 keeps a delta's `## Purpose` on archive, and strict
validation fails a change with no deltas (both tried on a temporary copy).
Strict validation also fails a requirement over 500 characters, so the field
and operator lists were split across requirements, adding B0-MODEL-004 and
B0-QUERY-005.

Verification of the revision:
- Strict OpenSpec validation: change valid.
- Structural audit: 12 capabilities, 29 requirements, 38 unique B0 ids; the
  folder is 386 lines.
- Command: `BASE_REF=c1faba9 scripts/check.sh ci`.
  Summary: `check ci: PASSED — 14 steps, 0 skipped, 25s @ c1faba9`.
  Same limits as above: untracked files, not committed-PR evidence.

## 2026-11-19: task 2.4 revision review
Reviewer: Codex (OpenAI GPT-6), independent standards and spec agents; reviser: Claude (Anthropic).
Scope: current untracked proposal against the public design and repository rules; no PR exists.
Standards: one blocker resolved by restoring B0-AGG-003; harness examples do not permit renumbering.
Spec: P1 resolved in service-api by requiring request propagation and adding B0-API-003:
security restrictions survive a user OR group; malformed security filters fail without SQL.
Spec: P2 resolved in contract-query and B0-AGG-003: invalid queries return query errors,
recognized out-of-B0 features return unsupported; invalid bundles first return model issues.
Verification: strict validation passed; 12 capabilities, 30 requirements, 39 unique ids, no main specs.
Command: `BASE_REF=c1faba9 scripts/check.sh ci`.
Summary: `check ci: PASSED — 14 steps, 0 skipped, 20s @ c1faba9`.
Evidence covers the working tree, not a committed PR; product tests remain zero; T1/T2 not run.
Open: prerequisite publication, the five gate-1 owner choices and owner approval;
the six deferred design questions remain for implementation proposals. No archive is authorized.
Learned: service-boundary security needs its own scenario; error classes must agree across capabilities.

## 2026-11-20: publication started
By Claude (Anthropic) through omp; GitHub writes used the implementer token.
- Issue #5 closed with a comment recording the decision.
- Pull request #6, the contract document alone, is merged.
- The harness branch could not be pushed with the implementer token (it changes a
  workflow file); the owner pushed it and it merged as #9.
- This folder is committed alone on a branch from main; the kernel document and config
  are a separate pull request. Both branches include #9 and pass default CI. The folder
  exceeds 400 lines.
- The first pull requests, #7 and #8, were closed and resubmitted from the same branches:
  their gate runs could no longer pass, and the implementer token cannot cancel runs.
Open: gate 1 for both; then `add-contract-model`, drafted on a local branch.
Learned: on both pull requests, a push made after the gate reached the approval wait
left the new gate run queued behind the old one. Finish every commit before opening.

## Bundle requirement transfer
The complete immutable model bundle requirement and B0-MODEL-003 move to add-versioned-model-bundles. This empties the contract-model delta, which is removed; existing main model requirements are unaffected.

## Service core transfer
B0-API-001/002/003 move to add-versioned-service-api. Network transport and worker dispatch remain proposed as B0-API-004; no gRPC acceptance is claimed.

## Network transfer
Complete Network service transport and B0-API-004 move to add-grpc-service. The emptied service-api seed delta is deleted.

## Doris temporal execution acceptance
B0-DORIS-010 remains proposed here after add-doris-time-queries implements and
locally executes the temporal feature. A separate harness change must integrate
its live-compiled test into the defined execution layer/tier before formal T2
acceptance. Product PR #58 does not archive this execution requirement.

## Doris temporal execution transfer
The complete Doris temporal execution tier requirement and B0-DORIS-010 move to
accept-doris-temporal-execution, after the Doris container tier began executing
the live-compiled manifest. B0-DORIS-001 and B0-DORIS-002 remain proposed here.

## Doris B0 acceptance transfer
The complete Doris B0 compilation and Doris B0 acceptance requirements, with
B0-DORIS-001 and B0-DORIS-002, move to add-doris-b0-acceptance. The emptied
dialect-doris seed delta is deleted. Presto and Druid deltas remain proposed.

## Presto execution cases
add-presto-b0-acceptance adds forty Presto execution cases that carry
B0-PRESTO-003. The requirement stays proposed here, reworded to literal expected
rows as Doris was, until the Presto tier (a harness path) runs the manifest and
passes on GitHub. B0-PRESTO-002 (T1) is untouched.

## Presto execution acceptance transfer
B0-PRESTO-003 moves to complete-compile-contract as its own requirement, after
the Presto tier ran the manifest on GitHub (#72). The remaining seed requirement
is reduced to the Cube Core comparison, B0-PRESTO-002 (T1). Druid deltas stay
proposed; the owner paused new dialects on 2026-11-23.

## Presto Cube Core comparison transfer
B0-PRESTO-002 moves to add-joined-measures, reworded to what the tier does, after
the comparison ran on GitHub as `cube-core` (#79). The emptied dialect-presto
seed delta is deleted. Only the Druid delta remains here; new dialects are
paused by the owner's decision of 2026-11-23.

## 2026-11-25: Druid compilation transferred

`add-druid-b0` implements Druid compilation: B0-DRUID-001 and B0-DRUID-002 are
transferred there, reworded from what was established on Druid 26.0.0 (joins
compile and run; a declared set of features is refused by name). The seed
keeps the two acceptance scenarios, B0-DRUID-003 and B0-DRUID-004, until
their tiers exist.


## 2026-11-25: Druid acceptance transferred; the seed is complete

`add-druid-approx-and-granularities` takes B0-DRUID-003 and B0-DRUID-004 into
`dialect-druid`, after the Druid tier ran both on GitHub as `druid-smoke` (#98,
#99). B0-DRUID-003 is reworded: a case may state that Cube Core refuses it or
writes SQL Druid rejects, and the tier verifies that. Nothing is left proposed
here, so this change is archived without deltas of its own. `scripts/dev` still
names this change's `design.md` as review context; it tolerates the missing
path, and the reference can go in a harness change.
