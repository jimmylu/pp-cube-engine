# Tasks

These are proposal preparation tasks, each bounded to two hours. They do not authorize product implementation or archive.

## 1. Prepare the contract
- [x] 1.1 Record owner decision #5 on deferred archive in notes.md; verify the entry links the decision issue.
- [x] 1.2 Compare the English public edition with source contract/kernel chapters; retain historical-evidence caveats and neutral names; verify by a search for private names.
- [x] 1.3 Add project context and artifact rules; verify OpenSpec reads config.yaml.
- [x] 1.4 Draft twelve capability deltas; verify proposal paths and unique scenario ids match.
- [x] 1.5 Document transfer of requirements to later implementing changes; verify no main specs are created.

## 2. Verify and review
- [x] 2.1 Run strict OpenSpec validation and repository checks; record exact scope and results in notes.md.
- [x] 2.2 Obtain a different-vendor review of the first draft; record the report in notes.md.
- [x] 2.3 Resolve the review findings; verify with strict validation, the id audit and the repository checks.
- [x] 2.4 Obtain a review of the revision from a vendor other than the reviser's; record the report and resolve findings.

## Workflow follow-up
- Publish the pending harness prerequisite and split public design/config PRs before the change-folder-only proposal PR.
- The owner approves and merges; do not claim step 5 approved before that gate. Gate 1 includes the choices listed in design.md.
- Implement each capability through a separate approved change with meaningful scenario tests, then archive its completed requirements.
