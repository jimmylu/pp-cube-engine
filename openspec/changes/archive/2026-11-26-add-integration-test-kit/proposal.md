# Proposal

## Why
Everything so far was verified on disposable containers in CI. Before more is built, the owner wants the kernel tested by hand in a real environment: real clusters, real engines, real models. The owner asked which of two ways to do that: a console with model editing and a drag-and-drop query builder, or testing through the API with a detailed guide. The recommendation the owner approved on 2026-11-26 is the second, with a command-line kit beside the guide, and a reduced console later only if non-engineers need one.

## What Changes
- `tools/integration-kit`: `ppcube-itest`, a Python command-line kit on the existing client. It checks an environment, shows a model version, compiles a query and prints the request, SQL, parameters, session properties and fingerprint, executes on Presto, Doris or Druid as a caller's back end would, loads and removes benchmark data, and runs a suite with expected answers into a report.
- `docs/integration-testing.zh-CN.md`: the guide, in Chinese, in the order of execution: environment sheet, release, deployment, smoke, benchmark track, real-model track, row-level security, model version lifecycle, replicas and upgrade, engine specifics, refusals and errors, record and conclusion.
- `examples/insurance-analytics/warehouse/insurance.json`: the example's rows as engine-neutral data.
- Rust tests tie the kit's data, the suite's expectations for each dialect and the guide's commands to the repository.

## Capabilities
### New Capabilities
- `integration-kit`: the kit and its guide.
### Modified Capabilities
None.

## Impact
New directory `tools/integration-kit`, one document, one data file, one test file, `.gitignore`, README and changelog. No change to the kernel, the service or the contract. The kit's own Python tests run locally in this change; running them in CI is a harness change.
