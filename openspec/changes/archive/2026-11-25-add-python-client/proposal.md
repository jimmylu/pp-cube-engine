# Proposal

## Why
Python is the second language of the kernel's callers, after Java: notebooks, data pipelines and services that ask the same questions as the portal. They have a `.proto` file. They need what Java callers got: a client that makes the right use the easy one, and something that runs.

## What Changes
- `clients/python`: a client for Python 3.9 and later, module `ppcube`. The gRPC stubs are generated from the service's contract when the package is built; a small layer covers Compile and GetMeta.
- The runnable example gains a Python back end beside the Java one. `examples/insurance-analytics/run.sh python` runs the same stack, the same users, rules and questions, and checks the same answers, executing on Presto through `presto-python-client`.
- A Rust test ties the Python client and back end to the contract and to the Java twin.

## Capabilities
### New Capabilities
- `client-python`: the Python client and its part of the example.
### Modified Capabilities
None.

## Impact
New directory `clients/python`, `examples/insurance-analytics/app-python`, the example's `run.sh` and README, one test, `.gitignore`, guide, README and changelog. No SQL changes and no change to the contract. The Python code is built and run locally in this change; running it in CI is a harness change that follows.
