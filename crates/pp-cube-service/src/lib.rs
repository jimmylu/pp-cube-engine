//! Synchronous versioned service core. Does not execute SQL or start a server.
use std::{
    collections::BTreeMap,
    fmt,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use pp_cube_compile::{CompileContext, CompileError, Compiled, DialectId};
use pp_cube_model::{
    BundleError, DimensionType, MeasureType, Model, ModelBundle, ModelCatalog, Query, Relationship,
    ValidationIssue,
};

/// Explicit model-selection or compilation failure; never a partial SQL response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceError {
    Bundle(BundleError),
    Compile(CompileError),
}
impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bundle(error) => error.fmt(f),
            Self::Compile(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for ServiceError {}

/// Discovery fields only: never SQL or physical sources.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct MeasureMeta {
    pub name: String,
    pub member_type: MeasureType,
    pub title: Option<String>,
    pub description: Option<String>,
    pub format: Option<String>,
    /// The model's `meta`, as written.
    pub meta: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct DimensionMeta {
    pub name: String,
    pub member_type: DimensionType,
    pub title: Option<String>,
    pub description: Option<String>,
    pub primary_key: bool,
    pub meta: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct JoinMeta {
    pub cube: String,
    pub relationship: Relationship,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct CubeMeta {
    pub name: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub data_source: String,
    pub measures: Vec<MeasureMeta>,
    pub dimensions: Vec<DimensionMeta>,
    pub segments: Vec<String>,
    /// Joins to public cubes only; a join to a hidden cube is not discoverable.
    pub joins: Vec<JoinMeta>,
    pub meta: Option<serde_json::Value>,
}

/// The outcome of publishing a version.
#[derive(Debug, Clone)]
pub struct Published {
    pub bundle: Arc<ModelBundle>,
    /// False when this exact content was already available under the version.
    pub created: bool,
}

/// What produced a result: the kernel build and the selected model version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub kernel_version: &'static str,
    pub upstream_pin: &'static str,
    pub model_version: String,
    pub model_digest: String,
}

/// Owns available validated versions. Visibility controls discovery, not authorization.
/// Calls are synchronous; a network host must arrange its own worker dispatch.
#[derive(Debug, Default)]
pub struct CubeService {
    /// Readers clone a bundle handle and release the lock before compiling.
    catalog: RwLock<ModelCatalog>,
    /// Which dialect serves each data source. A deployment fact, not model content.
    data_sources: BTreeMap<String, DialectId>,
}
impl CubeService {
    /// `data_sources` names the dialect of every data source the registered models may use.
    pub fn new(data_sources: impl IntoIterator<Item = (String, DialectId)>) -> Self {
        Self {
            catalog: RwLock::default(),
            data_sources: data_sources.into_iter().collect(),
        }
    }

    // A panic while holding the lock cannot leave the map half-updated: every
    // write is one insert or one remove.
    fn read(&self) -> RwLockReadGuard<'_, ModelCatalog> {
        self.catalog.read().unwrap_or_else(|e| e.into_inner())
    }
    fn write(&self) -> RwLockWriteGuard<'_, ModelCatalog> {
        self.catalog.write().unwrap_or_else(|e| e.into_inner())
    }
    fn bundle(&self, version: &str) -> Result<Arc<ModelBundle>, BundleError> {
        self.read().get(version)
    }

    /// Make a model version available, validating it first. A version is immutable:
    /// publishing the same content again changes nothing and reports `created: false`;
    /// different content under an existing version is refused. `expected_digest`, when
    /// given, must equal the content digest. Other versions are never affected, and a
    /// model on a data source this service has no dialect for is refused.
    pub fn publish(
        &self,
        version: impl Into<String>,
        model: Model,
        expected_digest: Option<&str>,
    ) -> Result<Published, BundleError> {
        // Validation precedes the digest so an invalid model reports its issues.
        let issues = pp_cube_model::validate_model(&model);
        if !issues.is_empty() {
            return Err(BundleError::Model(issues));
        }
        let digest = pp_cube_compile::model_digest(&model)
            .map_err(|_| BundleError::InvalidIdentifier { field: "digest" })?;
        if expected_digest.is_some_and(|expected| expected != digest) {
            return Err(BundleError::DigestMismatch);
        }
        let bundle = ModelBundle::new(version, digest, model)?;
        if let Some(cube) = bundle
            .model()
            .cubes
            .iter()
            .find(|cube| !self.data_sources.contains_key(&cube.data_source))
        {
            return Err(BundleError::UnmappedDataSource {
                data_source: cube.data_source.clone(),
            });
        }
        let mut catalog = self.write();
        match catalog.get(bundle.version()) {
            Ok(existing) if existing.digest() == bundle.digest() => Ok(Published {
                bundle: existing,
                created: false,
            }),
            Ok(_) => Err(BundleError::DuplicateVersion),
            Err(_) => Ok(Published {
                bundle: catalog.register(bundle)?,
                created: true,
            }),
        }
    }

    /// Stop offering a version. Compilations that already selected it finish on it.
    pub fn retire(&self, version: &str) -> Result<Arc<ModelBundle>, BundleError> {
        self.write().retire(version)
    }

    /// Available versions with their content digests, ordered by version.
    pub fn versions(&self) -> Vec<(String, String)> {
        self.read()
            .bundles()
            .iter()
            .map(|bundle| (bundle.version().to_owned(), bundle.digest().to_owned()))
            .collect()
    }

    /// Validate without registering or modifying any available version.
    pub fn validate_model(&self, model: &Model) -> Vec<ValidationIssue> {
        pp_cube_model::validate_model(model)
    }

    pub fn identity(&self, version: &str) -> Result<Identity, BundleError> {
        let bundle = self.bundle(version)?;
        Ok(Identity {
            kernel_version: pp_cube_compile::KERNEL_VERSION,
            upstream_pin: pp_cube_compile::UPSTREAM_PIN,
            model_version: bundle.version().to_owned(),
            model_digest: bundle.digest().to_owned(),
        })
    }

    /// Forward the complete trusted context unchanged, including security filters.
    /// SQL is returned only when the requested dialect serves every data source it reads,
    /// including cubes that only a security filter brought in.
    pub fn compile(
        &self,
        version: &str,
        query: &Query,
        context: &CompileContext,
    ) -> Result<Compiled, ServiceError> {
        let bundle = self.bundle(version).map_err(ServiceError::Bundle)?;
        let compiled = pp_cube_compile::compile(bundle.model(), query, context)
            .map_err(ServiceError::Compile)?;
        for used in &compiled.used_cubes {
            let source = bundle
                .model()
                .cube(&used.name)
                .map(|cube| cube.data_source.as_str());
            let served = source.and_then(|source| self.data_sources.get(source));
            if served != Some(&context.dialect) {
                return Err(ServiceError::Compile(CompileError::Query(format!(
                    "Cube `{}` is on data source `{}`, which the dialect `{}` does not serve",
                    used.name,
                    source.unwrap_or_default(),
                    context.dialect.name()
                ))));
            }
        }
        Ok(compiled)
    }

    /// The data source of a cube of an available version, for result lineage.
    pub fn data_source(&self, version: &str, cube: &str) -> Option<String> {
        let bundle = self.bundle(version).ok()?;
        Some(bundle.model().cube(cube)?.data_source.clone())
    }

    /// Public cubes and members in model order; omitted visibility means public.
    pub fn get_meta(&self, version: &str) -> Result<Vec<CubeMeta>, BundleError> {
        let bundle = self.bundle(version)?;
        let cubes = &bundle.model().cubes;
        let public = |name: &str| {
            cubes
                .iter()
                .any(|c| c.name == name && c.public != Some(false))
        };
        Ok(cubes
            .iter()
            .filter(|c| c.public != Some(false))
            .map(|c| CubeMeta {
                name: c.name.clone(),
                title: c.title.clone(),
                description: c.description.clone(),
                data_source: c.data_source.clone(),
                measures: c
                    .measures
                    .iter()
                    .filter(|m| m.public != Some(false))
                    .map(|m| MeasureMeta {
                        name: m.name.clone(),
                        member_type: m.measure_type,
                        title: m.title.clone(),
                        description: m.description.clone(),
                        format: m.format.clone(),
                        meta: m.meta.clone(),
                    })
                    .collect(),
                dimensions: c
                    .dimensions
                    .iter()
                    .filter(|d| d.public != Some(false))
                    .map(|d| DimensionMeta {
                        name: d.name.clone(),
                        member_type: d.dimension_type,
                        title: d.title.clone(),
                        description: d.description.clone(),
                        primary_key: d.primary_key,
                        meta: d.meta.clone(),
                    })
                    .collect(),
                segments: c.segments.iter().map(|s| s.name.clone()).collect(),
                joins: c
                    .joins
                    .iter()
                    .filter(|j| public(&j.name))
                    .map(|j| JoinMeta {
                        cube: j.name.clone(),
                        relationship: j.relationship,
                    })
                    .collect(),
                meta: c.meta.clone(),
            })
            .collect())
    }
}

mod workers;
pub use workers::{CompilePool, CompileTask, PoolError};

pub mod grpc;
