# Notes

## Done

- The Java client, the example and the contract's Java option carry the contract's own name; no file holds the stand-in any more (a byte search of the tracked files finds none).
- B1-CLIENT-002 requires every Java file to sit where its package says.

## Evidence

- `check java-client: PASSED — 5 tests`, built from a clean state with the old artifact removed from the local Maven repository.
- `check java-example: PASSED`, 23 questions asked, 20 answered as expected, 3 refused as expected, on real containers with the image rebuilt from the changed contract.
- The remote `java-client` and `java-example` runs on the delivered head are the evidence of record.

## Learned

- A name is not behaviour. Unit tests, an end-to-end run and CI all passed on a package named after a word the project does not use.
- `javac` does not require a source file's directory to match its package when the file is passed explicitly, which is how Maven compiles.
- Check new identifiers against the repository by bytes before publishing.

## Open

- The Maven group `dev.ppcube` is still a placeholder until the owner decides where the artifact is published.
