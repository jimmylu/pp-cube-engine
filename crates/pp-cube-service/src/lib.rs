//! Synchronous versioned service core. Does not execute SQL or start a server.
use std::{fmt, sync::Arc};

use pp_cube_compile::{CompileContext, CompileError, Compiled};
use pp_cube_model::{
    BundleError, DimensionType, MeasureType, Model, ModelBundle, ModelCatalog, Query,
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

/// Discovery fields only: deliberately excludes SQL, sources and opaque metadata.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct MemberMeta<T> {
    pub name: String,
    pub member_type: T,
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CubeMeta {
    pub name: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub measures: Vec<MemberMeta<MeasureType>>,
    pub dimensions: Vec<MemberMeta<DimensionType>>,
}

/// Owns available validated versions. Visibility controls discovery, not authorization.
/// Calls are synchronous; a network host must arrange its own worker dispatch.
#[derive(Debug, Default)]
pub struct CubeService {
    catalog: ModelCatalog,
}
impl CubeService {
    /// Validate before availability; invalid or duplicate versions leave existing ones intact.
    pub fn register(
        &mut self,
        version: impl Into<String>,
        digest: impl Into<String>,
        model: Model,
    ) -> Result<Arc<ModelBundle>, BundleError> {
        self.catalog
            .register(ModelBundle::new(version, digest, model)?)
    }

    /// Validate without registering or modifying any available version.
    pub fn validate_model(&self, model: &Model) -> Vec<ValidationIssue> {
        pp_cube_model::validate_model(model)
    }

    /// Forward the complete trusted context unchanged, including security filters.
    pub fn compile(
        &self,
        version: &str,
        query: &Query,
        context: &CompileContext,
    ) -> Result<Compiled, ServiceError> {
        let bundle = self.catalog.get(version).map_err(ServiceError::Bundle)?;
        pp_cube_compile::compile(bundle.model(), query, context).map_err(ServiceError::Compile)
    }

    /// Public cubes and members in model order; omitted visibility means public.
    pub fn get_meta(&self, version: &str) -> Result<Vec<CubeMeta>, BundleError> {
        let bundle = self.catalog.get(version)?;
        Ok(bundle
            .model()
            .cubes
            .iter()
            .filter(|c| c.public != Some(false))
            .map(|c| CubeMeta {
                name: c.name.clone(),
                title: c.title.clone(),
                description: c.description.clone(),
                measures: c
                    .measures
                    .iter()
                    .filter(|m| m.public != Some(false))
                    .map(|m| MemberMeta {
                        name: m.name.clone(),
                        member_type: m.measure_type,
                        title: m.title.clone(),
                        description: m.description.clone(),
                    })
                    .collect(),
                dimensions: c
                    .dimensions
                    .iter()
                    .filter(|d| d.public != Some(false))
                    .map(|d| MemberMeta {
                        name: d.name.clone(),
                        member_type: d.dimension_type,
                        title: d.title.clone(),
                        description: d.description.clone(),
                    })
                    .collect(),
            })
            .collect())
    }
}

mod workers;
pub use workers::{CompilePool, CompileTask, PoolError};

pub mod grpc;
