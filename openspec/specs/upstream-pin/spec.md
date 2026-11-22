# upstream-pin Specification

## Purpose
Keep planner behavior reproducible and tie every compiled result to the upstream commit behind it.
## Requirements
### Requirement: Pinned unmodified planner
The engine SHALL use the unmodified planner at the repository's pinned Cube commit. The upstream pin the kernel reports in fingerprints SHALL be that commit.
#### Scenario: B0-UPSTREAM-001 Pin identity
- **WHEN** the kernel is built from a checkout whose upstream directory is at the pinned commit
- **THEN** the upstream pin the kernel carries equals that commit, so moving the pin without updating the kernel fails the test
