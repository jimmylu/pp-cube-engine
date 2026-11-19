//! Implements the bridge interfaces the upstream planner requires.

/// The planner is used as it is at the commit pinned by the `vendor/cube`
/// submodule. Nothing under `vendor/` is edited in this repository.
pub use cubesqlplanner as upstream;
