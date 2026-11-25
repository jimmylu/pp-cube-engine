# Notes

## Done

- `clients/java`: the client and five unit tests against a stand-in service.
- `examples/insurance-analytics`: runs end to end with one command and checks its own answers.
- B1-CLIENT-001 and B1-CLIENT-002 keep the example and the client's build true to the repository under T0.

## Evidence

- Local, real containers (the service image built from this checkout, Presto 0.296 by digest), Java 21, Maven 3.9: `23 questions asked: 20 answered as expected, 3 refused as expected, 5 served from the cache` and `example: PASSED`. The run before it, without the paged question, also passed.
- `mvn -f clients/java/pom.xml test`: 5 tests pass.
- Not run in CI: the Java build and the example. Only the two Rust tests run there.

## Learned

- A ratio of two facts written as one measure is a query error ("no join path"), not `unsupported`. The example first expected the wrong kind; the Rust test caught it before the example ran. It now shows one refusal of each kind.
- None of the scenario's questions needs a session property on Presto; a paged question was added so that the example applies one.
- The example leaves a private key under `.run/` unless it removes it, and `gitleaks dir .` reads ignored files too. `run.sh` removes `.run/` when it stops.
- The first run takes about five minutes, most of it the image build; a change to the contract rebuilds the image.

## Open

- CI for Java: a harness change.
- Where the client artifact is published.
- Python and Node.js clients.
