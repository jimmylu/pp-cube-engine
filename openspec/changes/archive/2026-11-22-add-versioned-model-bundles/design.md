## Context
Model is already a complete Cube model set, Clone and PartialEq. compile consumes shared Arc<Model>. A small model-layer ownership boundary can add version management without altering the planner or introducing network transport.

## Decisions
ModelBundle owns private version/digest strings and Arc<Model>. Construction validates identifiers then runs validate_model. Getters return borrowed identifiers and a shared Arc reference; consumers can clone the Arc but copy-on-write cannot mutate bundle contents. Catalog registration consumes a validated bundle and returns an Arc<ModelBundle>; lookup clones that handle. No mutation/replacement/removal operation is exposed. HashMap keys are exact versions; duplicates fail even if digests match. &mut self makes writes explicit; external synchronization is a future service concern.

Caller digests are opaque, nonblank provenance labels, not cryptographic attestations. They neither bypass validation nor replace semantic compile fingerprints. Errors distinguish invalid identifiers, located model issues, duplicate and unknown versions; Display omits model/source data. No implicit latest version is introduced.

Alternatives: a service-owned registry would postpone a useful transport-independent product API; mutable replacement could silently change active queries; digest-based deduplication would trust unverified metadata. Persistent storage and automatic activation are outside this slice.

## Validation and risks
Public API integration tests cover invalid/duplicate registration, exact lookup and handles outliving the catalog. Compile integration alternates differently defined versions and compares complete repeated results without new snapshots. Tests name B0-MODEL-003/007/008. Existing compiler and Docker smoke gates remain unchanged. Arc handles can retain memory after catalog drop by design; no eviction policy is promised.
