# Proposal

## Why
The Java client and the example of `add-java-client` (#105) name their packages, the client class, the contract's `java_package` and the example's environment variables after a word the project uses nowhere else. Their directories carry the project's name. A Java package that does not match its directory builds and runs, so nothing failed; a Python package of the same name does not import, which is how it was found.

The cause was the implementing agent's: it worked under a stand-in for the project's name and wrote that stand-in into identifiers.

## What Changes
- Java packages become `dev.ppcube.client`, `dev.ppcube.engine.v1` and `dev.ppcube.example`, after the contract's own package `ppcube.engine.v1`; their directories follow.
- The client class becomes `CubeEngineClient`, after the service it calls.
- The Maven group becomes `dev.ppcube`.
- The example's environment variables become `PPCUBE_IMAGE`, `PPCUBE_TARGET`, `PPCUBE_CA` and `PPCUBE_TOKEN_FILE`.
- A test requires every Java file to sit where its package says, under `dev.ppcube`.

## Capabilities
### New Capabilities
None.
### Modified Capabilities
- `client-java`: the client and the example carry the contract's own name.

## Impact
`clients/java`, `examples/insurance-analytics`, one option in `crates/pp-cube-service/proto/engine.proto`, one test, and the archived documents of `add-java-client` where they named the class. The wire is unchanged. Nothing of this was released: 0.2.0 does not contain the client.
