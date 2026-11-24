# Spec Delta

## ADDED Requirements

### Requirement: Model store layout
A model store SHALL hold versions at `<business line>/models/<version>/`, where the business line has one to three levels. `models` is the asset type the service reads; keys under another name at that place SHALL be left alone. A version is complete when its `manifest.json` exists, and is named to callers by its business line and version joined by `/`. A `models` directory out of place and a malformed version name SHALL be reported, not guessed at.
#### Scenario: B1-STORE-001 Business lines and asset types
- **WHEN** a store holds complete versions under one, two and three levels of business line, a version without a manifest, another asset type, stray files, and `models` at the root, under four levels, under an upper-case line and with a malformed version
- **THEN** the three complete versions are found under their line and version, the incomplete one and the other asset type are left alone, and each misplaced or malformed one is reported as a layout fault

### Requirement: Versions from a store
The service SHALL offer every complete version of its store whose files are a model directory inside the contract, on data sources it has a dialect for, and whose content equals the manifest's digest when one is given. A version that cannot be offered SHALL be reported with one reason of a fixed set and SHALL NOT stop the others. Reports, logs and metrics SHALL name versions, files and rules, never model content.
#### Scenario: B1-STORE-002 Offered and rejected versions
- **WHEN** a directory store holds two valid versions, one with a matching digest, and versions with a wrong digest, an unknown manifest schema, a manifest that is not JSON, a stray file, a misnamed file, an invalid model and an unmapped data source
- **THEN** the valid ones compile by their business line and version, each faulty one is rejected with its own reason and counted by reason, and hidden entries and keys outside the store's root are not read

### Requirement: Following a store
Each later pass over the store SHALL offer versions that became complete and retire versions it offered whose manifest is gone. A loaded version SHALL NOT be read again. Versions made available another way SHALL NOT be retired by the store, and a pass that cannot list the store SHALL change nothing and be counted.
#### Scenario: B1-STORE-003 Added, retired and unreachable
- **WHEN** a version's files appear and then its manifest, a loaded version's files change, a manifest is removed, a store version is retired through the service, and the store becomes unreadable
- **THEN** the version is offered only with its manifest, the changed one is unaffected, the one without a manifest is retired while a version from elsewhere stays, the retired one returns at the next pass, and the failed pass leaves every version in place

### Requirement: Store in the process
The process SHALL take a directory as its model store and a refresh interval. It SHALL make the first pass before announcing its address and fail to start when the store cannot be listed; a version that cannot be offered SHALL NOT prevent starting.
#### Scenario: B1-API-002 A replica on a store
- **WHEN** the process starts on a store with one valid and one broken version, and while it runs a new version is published and the first one's manifest removed; and separately it is started on a missing directory, on a store of an unbuilt kind, and with a refresh interval but no store
- **THEN** the valid version compiles as soon as the address is announced, the new one compiles within the refresh interval, the retired one stops compiling, and the three faulty invocations exit without announcing an address
