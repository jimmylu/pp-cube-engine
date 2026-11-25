# Design

## Decisions

1. **Name after the contract.** The contract's package is `ppcube.engine.v1`, the metrics are `ppcube_…`, the service is `CubeEngine`. The Java names follow those: `dev.ppcube.*` and `CubeEngineClient`. No new word is introduced.
2. **`dev.` stays a placeholder group.** Where the artifact is published is still the owner's to decide; the group will follow that decision.
3. **A test for what the compiler does not check.** `javac` accepts a source file whose package differs from its directory when files are passed explicitly, as Maven does. B1-CLIENT-002 now reads every `.java` file of the client and the example and requires its package to begin with `dev.ppcube.` and to equal its path. It also requires the contract's `java_package`.
4. **The archived change is corrected in place** where it named the class, so that a reader of the history finds names that exist.

## How it was missed, and what changes for the implementer

Everything passed: the unit tests, the example on real containers, and both new CI jobs. Each of them proves behaviour, and behaviour was right. What was wrong was a name, and nothing compared names with the rest of the project.

The agent now checks identifiers it introduces against names already in the repository by reading bytes, not rendered text, before publishing.
