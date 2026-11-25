# Proposal

## Why
The callers of the kernel are mainly Java. Until now a Java caller had a `.proto` file and a Rust example. A caller needs a client that makes the right use the easy one (bind the parameters, apply the session properties, tell a refusal from a failed call), and something that runs and shows the whole path from a user's question to rows.

## What Changes
- `clients/java`: a Java 11 client. The gRPC stubs are generated from the service's own contract, and a small typed layer covers Compile and GetMeta: TLS and token, the compiled query with a JDBC binder, a refusal by kind.
- `examples/insurance-analytics`: a runnable example. An insurer's analytics portal asks its dashboards' questions as four users, one under a row-level rule. The service runs as the shipped deployment runs it, against a model store; the SQL is executed on Presto 0.296 through JDBC with bound parameters and session properties; every answer is checked against one worked out by hand.
- The contract names its Java package (`option java_package`); the wire is unchanged.
- Rust tests keep the example true to the property-insurance scenario and to the shipped deployment.

## Capabilities
### New Capabilities
- `client-java`: the Java client and the example that uses it.
### Modified Capabilities
None.

## Impact
New directories `clients/java` and `examples/insurance-analytics`; three option lines in `crates/pp-cube-service/proto/engine.proto`; one test file in `crates/pp-cube-conformance`; `.gitignore`; guide, README and changelog. No SQL changes. The Java code is built and run locally in this change; building it in CI is a harness change that follows.
