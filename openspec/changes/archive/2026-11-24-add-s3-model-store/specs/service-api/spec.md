# Spec Delta

## MODIFIED Requirements

### Requirement: Following a store
Each later pass over the store SHALL offer versions that became complete and retire versions it offered whose manifest is gone. A loaded version SHALL NOT be read again. Versions made available another way SHALL NOT be retired by the store, and a pass that cannot list the store SHALL change nothing and be counted. A version that stays rejected for the same reason SHALL be counted in every pass and logged once.
#### Scenario: B1-STORE-003 Added, retired and unreachable
- **WHEN** a version's files appear and then its manifest, a loaded version's files change, a manifest is removed, a store version is retired through the service, the store becomes unreadable, and a rejected version is passed over again with the same and then with another reason
- **THEN** the version is offered only with its manifest, the changed one is unaffected, the one without a manifest is retired while a version from elsewhere stays, the retired one returns at the next pass, the failed pass leaves every version in place, and the rejection is logged again only when its reason changes

## ADDED Requirements

### Requirement: S3-protocol store
A model store MAY be an S3-protocol bucket under a prefix, with the layout of a directory store. The service SHALL list it with ListObjectsV2 over as many pages as it takes and read objects with signed requests, taking keys relative to the prefix and leaving out folder markers and hidden entries. A failure SHALL name what was asked and the HTTP status, never the server's text.
#### Scenario: B1-STORE-004 Bucket under a prefix
- **WHEN** a bucket holds two complete versions under a prefix with a folder marker, a hidden entry and keys of another prefix, its listing takes several pages, an object is missing, and the bucket then refuses requests
- **THEN** the two versions are offered and compile as from a directory, every request is signed for that bucket, the missing object and the refusal are reported by status alone, and the refused pass changes nothing

### Requirement: Store credentials
The service SHALL hold no storage secret and take none as an option. It SHALL sign with access keys from the standard environment variables, or without them with the instance role's credentials from the metadata service, asked with a session token and kept for a while. The region, a custom endpoint and the metadata endpoint SHALL come from their standard variables. Half a key pair and web identity credentials SHALL be refused by name.
#### Scenario: B1-STORE-005 Environment and instance role
- **WHEN** a store is configured from variables with keys, a session token, a region and two endpoints, from none, from half a key pair and from a web identity token file, and a store without keys is listed and read several times
- **THEN** the first uses the keys, region and the S3 endpoint, the second is AWS with the default region and the instance role, the others are refused, and the keyless store asks the metadata service once and signs with the role's key and session token

### Requirement: Several stores
The process SHALL take any number of model stores, each a directory or `s3://bucket/prefix`, read every one before announcing its address, and number them in its metrics. A bucket that cannot be listed at startup SHALL stop the process, and a store of another scheme SHALL be refused.
#### Scenario: B1-API-003 A directory and a bucket at once
- **WHEN** the process starts on a directory and a bucket, and separately on a bucket that does not exist and on a store of another scheme
- **THEN** the versions of both stores are loaded before the address is announced and each store's pass is counted under its number, no secret is logged, and the two faulty invocations exit without announcing an address
