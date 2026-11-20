# Tasks

Planning artifacts only in this session. Each implementation task is bounded to two hours; all tests below name `// scenario: B0-VALID-001`.

## 1. Located document diagnostics

- [x] 1.1 Write a failing public validation test, then add ValidationIssue and YAML/JSON entry points. Document pointer paths and stable rule ids; verify malformed syntax gives a root issue and a valid fixture gives an empty report.
- [x] 1.2 Add test-first traversal of required fields, types, enum values and field allowlists at each object level. Verify several independent faults are reported together, with exact paths/rules, while strict loaders retain their previous failures.
- [x] 1.3 Preserve metadata and JSON parser constraints. Verify equivalent YAML/JSON diagnostics, strict JSON extensions disabled, existing exact-number cases and deterministic report order.

## 2. Semantic model structure

- [x] 2.1 Add typed-model validation and shared source/name checks. Verify exactly one nonempty source, duplicate cube/member names, programmatic missing measure SQL and valid forward references through public APIs.
- [x] 2.2 Reuse reference grammar and validate all usable expression fields. Verify multiple unknown references, malformed reference locations and independent semantic errors beside raw shape failures; document cascade suppression.
- [x] 2.3 Validate join targets, both endpoint primary keys and uniqueness per unordered cube pair. Verify missing targets, each missing key, repeated/reciprocal declarations and combined independent failures without changing query-dependent join behavior.

## 3. Integration

- [x] 3.1 Run model and compile tests plus strict OpenSpec validation. Verify existing model fixtures pass, every part of B0-VALID-001 has meaningful assertions, and no existing snapshots, lockfile, vendor or harness paths changed; record results in notes.md.
- [x] 3.2 Run fast before checkpoint and CI before final review. Verify summary lines, update tasks/notes and publish SHA-bound evidence outside Git.

## Workflow follow-up

- Continue implementation in this branch's one Draft PR after an apply request.
- Archive the implemented structural requirement before final review in the same Draft PR, per docs/agents/workflow.md; retain deferred SQL/expression requirements in the seed.
- Obtain different-vendor review, then Ready; owner approves the final head and merges.
