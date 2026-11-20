# Tasks

Planning is complete; unchecked tasks are implementation work and do not authorize applying this change. Each task is bounded to two hours.

## 1. Context composition

- [ ] 1.1 Write a failing facade test naming `B0-SEC-001`, then add typed context security filters and top-level composition; document the field and update existing context literals. Verify grouped user OR, multiple security entries, conflicting same-member conditions, ordered parameters and unchanged inputs with `cargo test -p pp-cube-compile --test security`.
- [ ] 1.2 Add tests naming `B0-SEC-004`, then reject empty security AND/OR groups recursively. Verify root and nested empty groups fail without SQL and an empty context list preserves SQL/parameters; verify ordinary user-group behavior remains unchanged.

## 2. Restriction preservation and failures

- [ ] 2.1 Add the minimal fixture dimensions and golden tests naming `B0-SEC-001` and `B0-SEC-002`. Verify the exact channel/category/region grouping and mandatory cube predicate with absent and present user/context filters; inspect SQL and parameters together using `cargo insta review`.
- [ ] 2.2 Add tests naming `B0-SEC-003` for malformed filter deserialization, unknown members, incompatible operators, invalid counts and dates. Verify each returns an error with no compiled result; fix only reproduced failures within the stated facade/adapter scope.
- [ ] 2.3 Add tests naming `B0-SEC-003` for a security-only joined dimension two hops away and disconnected, ambiguous, cross-source and third-cube-condition refusals. Verify the successful query includes the required joins and predicate, while each prohibited tree returns no SQL.

## 3. Integration verification

- [ ] 3.1 Run compile-crate tests and strict OpenSpec validation. Verify no pre-existing snapshot, vendor file, lockfile or harness path changed; verify each of the four ids is named by meaningful tests and record results in notes.md.
- [ ] 3.2 Run `scripts/check.sh fast` before checkpoint and `scripts/check.sh ci` before final review. Verify the summary lines and record their scope; keep SHA-bound evidence outside Git at publication.

## Workflow follow-up

- Continue planning, implementation and eventual archive in this branch's single Draft PR; do not merge a separate planning PR.
- After implementation and verification, archive only these implemented requirements, retain the remaining seed backlog, and validate the resulting main spec and traceability.
- Use cross-vendor review before Ready; report commit, push status, PR URL and blockers. The owner approves the final head and merges.
