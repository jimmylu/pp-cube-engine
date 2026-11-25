# Design

## Decisions

1. **Stubs from the contract, not from a copy.** `clients/java/pom.xml` points the protobuf plugin at `crates/pp-cube-service/proto`. A client built from a checkout cannot drift from that checkout's service. The artifact carries the kernel's version; a Rust test fails when they differ.
2. **A thin layer, on the two calls a caller makes all day.** `CindervaleClient.compile` and `meta`. Model management stays on the generated stubs: it is an operator's tool with different credentials, and wrapping it would add surface without removing a mistake.
3. **The layer exists to remove four mistakes.**
   - Splicing values into SQL: `CompiledQuery.bind(PreparedStatement)` binds by type, in order.
   - Dropping session properties: they are a map on the result, named in every description of it.
   - Retrying a refusal: an answer without SQL is `CompileRefusedException` with a kind; a failed call stays `StatusRuntimeException`. The two are different types because the right reaction differs.
   - Sending a token in clear: `plaintext()` with a token does not build.
   An unknown parameter kind or failure kind from a newer service is never mapped to a known one: the first throws, the second is `INTERNAL`.
4. **Query and filters are JSON text.** A caller already holds a Cube query as JSON. A typed query builder would be a second definition of Cube's format to keep in step; the kernel validates the text and names what is wrong.
5. **Java 11, gRPC with shaded Netty.** The lowest version still common in the callers this is for; the shaded transport avoids a Netty conflict in the caller's application.
6. **The example is a deployment in small.** Its service takes the arguments of `deploy/kubernetes.yaml` (a test compares them), reads a model store in the published layout, and is reached over TLS with a token made for the run. Its warehouse is Presto 0.296 by the digest the engine tier uses. Its questions and expected answers are those of the property-insurance scenario, which were worked out by hand and run on the engines.
7. **The example checks itself.** `run.sh` ends with `example: PASSED` or a non-zero status. It can serve as the Java end-to-end check once CI can run it.
8. **`option java_package`.** Without it the generated classes land in `ppcube.engine.v1` inside one outer class. The options change nothing on the wire and nothing for Rust.

## Verification

- T0: Rust tests B1-CLIENT-001 and B1-CLIENT-002 tie the example's model, questions, expectations, service arguments and the client's build to the kernel, without a JVM.
- Local, in this change: the client's unit tests (`mvn test`) and the example end to end on real containers.
- Not in CI yet: `scripts/check.sh` and the workflows are harness paths. A harness change will add the Maven build and the example run.

## Not in this change

- Publishing the artifact to a repository. Which one is the owner's to decide.
- Python and Node.js clients.
- A client-side balancer over replicas; the deployment guide describes the choices.
