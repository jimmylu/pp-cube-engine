//! Validated immutable models and exact, in-memory version selection.
use std::collections::{hash_map::Entry, HashMap};
use std::fmt;
use std::sync::Arc;

use crate::{validate_model, Model, ValidationIssue};

/// Registration/selection failures. Digest strings are never used as proof of validity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BundleError {
    InvalidIdentifier {
        field: &'static str,
    },
    Model(Vec<ValidationIssue>),
    DuplicateVersion,
    UnknownVersion,
    /// A host that maps data sources to dialects has no dialect for this one.
    UnmappedDataSource {
        data_source: String,
    },
}

impl fmt::Display for BundleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier { field } => write!(f, "bundle {field} must not be blank"),
            Self::Model(_) => f.write_str("the bundle model could not be validated"),
            Self::DuplicateVersion => f.write_str("the bundle version is already registered"),
            Self::UnknownVersion => f.write_str("the bundle version is not available"),
            Self::UnmappedDataSource { data_source } => {
                write!(
                    f,
                    "no dialect is configured for data source `{data_source}`"
                )
            }
        }
    }
}
impl std::error::Error for BundleError {}

/// One complete, validated model set. No model or identity mutation is exposed.
#[derive(Debug)]
pub struct ModelBundle {
    version: String,
    digest: String,
    model: Arc<Model>,
}

impl ModelBundle {
    /// Own and validate a complete model set. Identifiers are exact nonblank strings.
    /// The caller's digest is opaque metadata, not a verified content hash.
    pub fn new(
        version: impl Into<String>,
        digest: impl Into<String>,
        model: Model,
    ) -> Result<Self, BundleError> {
        let version = version.into();
        let digest = digest.into();
        for (field, value) in [("version", &version), ("digest", &digest)] {
            if value.trim().is_empty() {
                return Err(BundleError::InvalidIdentifier { field });
            }
        }
        let issues = validate_model(&model);
        if !issues.is_empty() {
            return Err(BundleError::Model(issues));
        }
        Ok(Self {
            version,
            digest,
            model: Arc::new(model),
        })
    }

    pub fn version(&self) -> &str {
        &self.version
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }

    /// Suitable for the compile facade. Cloning this Arc cannot mutate this bundle.
    pub fn model(&self) -> &Arc<Model> {
        &self.model
    }
}

/// Exact version lookup; no implicit latest version, replacement, persistence or IO.
/// Writes require exclusive access; retained read handles outlive the catalog.
#[derive(Debug, Default)]
pub struct ModelCatalog {
    bundles: HashMap<String, Arc<ModelBundle>>,
}

impl ModelCatalog {
    /// Register an already validated bundle, rejecting every duplicate version.
    pub fn register(&mut self, bundle: ModelBundle) -> Result<Arc<ModelBundle>, BundleError> {
        match self.bundles.entry(bundle.version.clone()) {
            Entry::Occupied(_) => Err(BundleError::DuplicateVersion),
            Entry::Vacant(entry) => {
                let bundle = Arc::new(bundle);
                entry.insert(Arc::clone(&bundle));
                Ok(bundle)
            }
        }
    }

    pub fn get(&self, version: &str) -> Result<Arc<ModelBundle>, BundleError> {
        self.bundles
            .get(version)
            .cloned()
            .ok_or(BundleError::UnknownVersion)
    }
}
