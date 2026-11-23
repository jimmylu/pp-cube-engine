//! Host-mounted gRPC transport for trusted callers; no listener or authentication defaults.
use crate::{CompilePool, CubeService, PoolError, ServiceError};
use pp_cube_compile::{CompileContext, CompileError, DialectId, Parameter as CompiledParameter};
use pp_cube_model::{load_query, BundleError};
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub mod wire {
    tonic::include_proto!("ppcube.engine.v1");
}
use wire::*;

pub struct GrpcService {
    core: Arc<CubeService>,
    pool: CompilePool,
    management: bool,
}
impl GrpcService {
    pub fn new(core: Arc<CubeService>, workers: usize, queue: usize) -> Result<Self, PoolError> {
        Ok(Self {
            pool: CompilePool::new(Arc::clone(&core), workers, queue)?,
            core,
            management: false,
        })
    }

    /// Allow ListModels, PublishModel and RetireModel. Off by default: whoever may
    /// publish decides what SQL every caller gets, so the host must opt in and
    /// must control who can reach the service.
    pub fn with_management(mut self, enabled: bool) -> Self {
        self.management = enabled;
        self
    }

    fn manage(&self) -> Result<(), Status> {
        if self.management {
            Ok(())
        } else {
            Err(Status::permission_denied("model management is not enabled"))
        }
    }
}
fn internal() -> Status {
    Status::internal("service operation failed")
}
fn bundle_error(error: BundleError) -> Status {
    match error {
        BundleError::UnknownVersion => Status::not_found("model version not available"),
        BundleError::DuplicateVersion => {
            Status::already_exists("model version is available with different content")
        }
        BundleError::DigestMismatch => {
            Status::failed_precondition("model content does not match the expected digest")
        }
        BundleError::InvalidIdentifier { .. } => {
            Status::invalid_argument("model version must not be blank")
        }
        BundleError::UnmappedDataSource { data_source } => Status::failed_precondition(format!(
            "no dialect is configured for data source `{data_source}`"
        )),
        BundleError::Model(_) => internal(),
    }
}
fn parse_model(bytes: Vec<u8>) -> Result<String, Status> {
    String::from_utf8(bytes)
        .map_err(|_| Status::invalid_argument("model must be UTF-8 YAML or JSON"))
}
fn is_json(text: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(text).is_ok()
}
fn issues(issues: Vec<pp_cube_model::ValidationIssue>) -> Vec<ModelIssue> {
    issues
        .into_iter()
        .map(|i| ModelIssue {
            path: i.path,
            rule: i.rule_id,
            message: i.message,
        })
        .collect()
}
fn failure(kind: FailureKind, message: String, identity: Identity) -> Response<CompileResponse> {
    Response::new(CompileResponse {
        result: Some(compile_response::Result::Failure(CompileFailure {
            kind: kind.into(),
            message,
        })),
        identity: Some(identity),
    })
}
fn identity(core: &CubeService, version: &str) -> Result<Identity, Status> {
    let identity = core.identity(version).map_err(bundle_error)?;
    Ok(Identity {
        kernel_version: identity.kernel_version.into(),
        upstream_pin: identity.upstream_pin.into(),
        model_version: identity.model_version,
        model_digest: identity.model_digest,
    })
}
fn label<T: serde::Serialize>(value: T) -> Result<String, Status> {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .ok_or_else(internal)
}
fn meta_json(value: Option<serde_json::Value>) -> Result<Option<String>, Status> {
    value
        .map(|value| serde_json::to_string(&value).map_err(|_| internal()))
        .transpose()
}
#[tonic::async_trait]
impl cube_engine_server::CubeEngine for GrpcService {
    async fn compile(
        &self,
        request: Request<CompileRequest>,
    ) -> Result<Response<CompileResponse>, Status> {
        let request = request.into_inner();
        let query = load_query(&request.query_json)
            .map_err(|_| Status::invalid_argument("invalid query JSON"))?;
        let mut security_filters = Vec::new();
        for text in request.security_filters_json {
            // Each field represents one Cube filter, never a complete query document.
            let parsed: serde_json::Value = serde_json::from_str(&text)
                .map_err(|_| Status::invalid_argument("invalid security filter JSON"))?;
            let wrapped = serde_json::json!({"filters": [parsed]}).to_string();
            security_filters.extend(
                load_query(&wrapped)
                    .map_err(|_| Status::invalid_argument("invalid security filter"))?
                    .filters,
            );
        }
        // An unavailable version is NotFound before anything else is said about the request.
        let identity = identity(&self.core, &request.model_version)?;
        let dialect = match (
            DialectId::from_name(&request.dialect),
            request.dialect.as_str(),
        ) {
            (Some(dialect), _) => dialect,
            (None, "druid") => {
                return Ok(failure(
                    FailureKind::Unsupported,
                    "dialect is not implemented".into(),
                    identity,
                ))
            }
            _ => return Err(Status::invalid_argument("unknown dialect")),
        };
        let version = request.model_version.clone();
        let context = CompileContext {
            request_id: request.request_id,
            dialect,
            timezone: request.timezone,
            security_filters,
        };
        let task = self
            .pool
            .submit(request.model_version, query, context)
            .map_err(pool_status)?;
        match task.wait().await {
            Ok(result) => {
                let params = result
                    .parameters
                    .into_iter()
                    .map(|p| {
                        let value = match p {
                            CompiledParameter::Text(v) => parameter::Value::Text(v),
                            CompiledParameter::Bool(v) => parameter::Value::Boolean(v),
                            CompiledParameter::Null => parameter::Value::Null(Null {}),
                            CompiledParameter::Number(v) if v.is_finite() => {
                                parameter::Value::Number(v)
                            }
                            CompiledParameter::Number(_) => return Err(internal()),
                        };
                        Ok(Parameter { value: Some(value) })
                    })
                    .collect::<Result<Vec<_>, Status>>()?;
                let columns = result
                    .columns
                    .into_iter()
                    .map(|c| {
                        Ok(OutputColumn {
                            alias: c.alias,
                            member: c.member,
                            r#type: label(c.semantic_type)?,
                            granularity: c.granularity,
                        })
                    })
                    .collect::<Result<Vec<_>, Status>>()?;
                let used_cubes = result
                    .used_cubes
                    .into_iter()
                    .map(|c| {
                        Ok(UsedCube {
                            data_source: self
                                .core
                                .data_source(&version, &c.name)
                                .ok_or_else(internal)?,
                            name: c.name,
                            tables: c
                                .physical_tables
                                .into_iter()
                                .map(|path| Table { path })
                                .collect(),
                        })
                    })
                    .collect::<Result<Vec<_>, Status>>()?;
                Ok(Response::new(CompileResponse {
                    result: Some(compile_response::Result::Compiled(Compiled {
                        sql: result.sql,
                        params,
                        columns,
                        used_cubes,
                        fingerprint: result.fingerprint.to_vec(),
                        session_properties: result
                            .session_properties
                            .into_iter()
                            .map(|p| SessionProperty {
                                name: p.name,
                                value: p.value,
                            })
                            .collect(),
                    })),
                    identity: Some(identity),
                }))
            }
            Err(PoolError::Compile(ServiceError::Compile(error))) => {
                let kind = match &error {
                    CompileError::Model(_) => FailureKind::Model,
                    CompileError::Query(_) => FailureKind::Query,
                    CompileError::Unsupported(_) => FailureKind::Unsupported,
                    CompileError::Internal { .. } => FailureKind::Internal,
                };
                Ok(failure(kind, error.to_string(), identity))
            }
            Err(PoolError::Compile(ServiceError::Bundle(error))) => Err(bundle_error(error)),
            Err(error) => Err(pool_status(error)),
        }
    }
    async fn validate_model(
        &self,
        request: Request<ValidateModelRequest>,
    ) -> Result<Response<ValidateModelResponse>, Status> {
        let text = parse_model(request.into_inner().model_bundle)?;
        let (found, model_digest) = tokio::task::spawn_blocking(move || {
            let (found, model) = if is_json(&text) {
                (
                    pp_cube_model::validate_json(&text),
                    pp_cube_model::load_json(&text).ok(),
                )
            } else {
                (
                    pp_cube_model::validate_yaml(&text),
                    pp_cube_model::load_yaml(&text).ok(),
                )
            };
            let digest = model
                .filter(|_| found.is_empty())
                .and_then(|model| pp_cube_compile::model_digest(&model).ok());
            (found, digest)
        })
        .await
        .map_err(|_| internal())?;
        Ok(Response::new(ValidateModelResponse {
            issues: issues(found),
            model_digest,
        }))
    }
    async fn list_models(
        &self,
        _request: Request<ListModelsRequest>,
    ) -> Result<Response<ListModelsResponse>, Status> {
        self.manage()?;
        Ok(Response::new(ListModelsResponse {
            models: self
                .core
                .versions()
                .into_iter()
                .map(|(model_version, model_digest)| ModelVersion {
                    model_version,
                    model_digest,
                })
                .collect(),
            kernel_version: pp_cube_compile::KERNEL_VERSION.into(),
            upstream_pin: pp_cube_compile::UPSTREAM_PIN.into(),
        }))
    }
    async fn publish_model(
        &self,
        request: Request<PublishModelRequest>,
    ) -> Result<Response<PublishModelResponse>, Status> {
        self.manage()?;
        let request = request.into_inner();
        let text = parse_model(request.model_bundle)?;
        let core = Arc::clone(&self.core);
        let version = request.model_version;
        let outcome = tokio::task::spawn_blocking(move || {
            // Report located issues from the text; a model that does not load has them.
            let (found, model) = if is_json(&text) {
                (
                    pp_cube_model::validate_json(&text),
                    pp_cube_model::load_json(&text).ok(),
                )
            } else {
                (
                    pp_cube_model::validate_yaml(&text),
                    pp_cube_model::load_yaml(&text).ok(),
                )
            };
            match model {
                Some(model) if found.is_empty() => core
                    .publish(version.clone(), model, request.expected_digest.as_deref())
                    .map(|published| (published.created, version))
                    .map_err(Ok),
                _ => Err(Err(found)),
            }
        })
        .await
        .map_err(|_| internal())?;
        let result = match outcome {
            Ok((created, version)) => publish_model_response::Result::Published(Published {
                identity: Some(identity(&self.core, &version)?),
                created,
            }),
            Err(Err(found)) | Err(Ok(BundleError::Model(found))) => {
                publish_model_response::Result::Rejected(Rejected {
                    issues: issues(found),
                })
            }
            Err(Ok(error)) => return Err(bundle_error(error)),
        };
        Ok(Response::new(PublishModelResponse {
            result: Some(result),
        }))
    }
    async fn retire_model(
        &self,
        request: Request<RetireModelRequest>,
    ) -> Result<Response<RetireModelResponse>, Status> {
        self.manage()?;
        let bundle = self
            .core
            .retire(&request.into_inner().model_version)
            .map_err(bundle_error)?;
        Ok(Response::new(RetireModelResponse {
            identity: Some(Identity {
                kernel_version: pp_cube_compile::KERNEL_VERSION.into(),
                upstream_pin: pp_cube_compile::UPSTREAM_PIN.into(),
                model_version: bundle.version().to_owned(),
                model_digest: bundle.digest().to_owned(),
            }),
        }))
    }
    async fn get_meta(
        &self,
        request: Request<GetMetaRequest>,
    ) -> Result<Response<GetMetaResponse>, Status> {
        let core = Arc::clone(&self.core);
        let version = request.into_inner().model_version;
        let identity = identity(&self.core, &version)?;
        let meta = tokio::task::spawn_blocking(move || core.get_meta(&version))
            .await
            .map_err(|_| internal())?
            .map_err(bundle_error)?;
        let cubes = meta
            .into_iter()
            .map(|c| {
                Ok(CubeMeta {
                    name: c.name,
                    title: c.title,
                    description: c.description,
                    data_source: c.data_source,
                    measures: c
                        .measures
                        .into_iter()
                        .map(|m| {
                            Ok(MeasureMeta {
                                name: m.name,
                                r#type: label(m.member_type)?,
                                title: m.title,
                                description: m.description,
                                format: m.format,
                                meta_json: meta_json(m.meta)?,
                            })
                        })
                        .collect::<Result<_, Status>>()?,
                    dimensions: c
                        .dimensions
                        .into_iter()
                        .map(|d| {
                            Ok(DimensionMeta {
                                name: d.name,
                                r#type: label(d.member_type)?,
                                title: d.title,
                                description: d.description,
                                primary_key: d.primary_key,
                                meta_json: meta_json(d.meta)?,
                            })
                        })
                        .collect::<Result<_, Status>>()?,
                    segments: c.segments,
                    joins: c
                        .joins
                        .into_iter()
                        .map(|j| {
                            Ok(JoinMeta {
                                cube: j.cube,
                                relationship: label(j.relationship)?,
                            })
                        })
                        .collect::<Result<_, Status>>()?,
                    meta_json: meta_json(c.meta)?,
                })
            })
            .collect::<Result<Vec<_>, Status>>()?;
        Ok(Response::new(GetMetaResponse {
            cubes,
            identity: Some(identity),
        }))
    }
}
fn pool_status(error: PoolError) -> Status {
    match error {
        PoolError::Busy => Status::resource_exhausted("compilation queue is full"),
        PoolError::Unavailable => Status::unavailable("compilation worker unavailable"),
        _ => internal(),
    }
}
