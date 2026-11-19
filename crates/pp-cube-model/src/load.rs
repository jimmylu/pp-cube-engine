//! Loading a model document into the contract types.

use std::fmt;

use crate::model::Model;

/// A place in a document; lines and columns start at 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

/// Why a document did not load. Loading stops at the first problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadError {
    pub message: String,
    /// Where the problem is, when the parser can tell.
    pub position: Option<Position>,
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for LoadError {}

/// Loads a model written in YAML.
pub fn load_yaml(text: &str) -> Result<Model, LoadError> {
    serde_norway::from_str(text).map_err(|error| LoadError {
        position: error.location().map(|at| Position {
            line: at.line(),
            column: at.column(),
        }),
        message: error.to_string(),
    })
}

/// Loads a model written in JSON.
pub fn load_json(text: &str) -> Result<Model, LoadError> {
    serde_json::from_str(text).map_err(|error| LoadError {
        position: Some(Position {
            line: error.line(),
            column: error.column(),
        }),
        message: error.to_string(),
    })
}
