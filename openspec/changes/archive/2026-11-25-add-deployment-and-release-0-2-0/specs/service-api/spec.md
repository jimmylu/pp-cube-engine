# Spec Delta

## ADDED Requirements

### Requirement: Shipped deployment
The repository SHALL hold a deployment of three replicas whose service arguments start a serving replica as written. It SHALL replace one replica at a time and keep at least two available, probe a port that answers only once models are validated, allow longer to stop than the service drains, run as a non-root user with a read-only filesystem, and name the image of the current version, never a mutable tag.
#### Scenario: B1-API-004 The manifest's arguments
- **WHEN** the service is started with the arguments of the shipped manifest, its mounted certificate, tokens and model store made available, and is then sent SIGTERM
- **THEN** it announces that it listens, the probed path answers 200 and reports the store's model version, and it exits successfully; and the manifest states three replicas, one unavailable, at least two available, a grace period longer than the drain, and the current version's image
