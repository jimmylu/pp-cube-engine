//! Loading a model document into the contract types.

use std::fmt;

use crate::model::Model;

/// Why a document did not load. Loading stops at the first problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadError {
    pub message: String,
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
        message: error.to_string(),
    })
}

/// Loads a model written in JSON.
pub fn load_json(text: &str) -> Result<Model, LoadError> {
    serde_json::from_str(text).map_err(|error| LoadError {
        message: error.to_string(),
    })
}
