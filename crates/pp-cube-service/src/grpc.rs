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
}
impl GrpcService {
    pub fn new(core: Arc<CubeService>, workers: usize, queue: usize) -> Result<Self, PoolError> {
        Ok(Self {
            pool: CompilePool::new(Arc::clone(&core), workers, queue)?,
            core,
        })
    }
}
fn internal() -> Status {
    Status::internal("service operation failed")
}
fn bundle_error(error: BundleError) -> Status {
    match error {
        BundleError::UnknownVersion => Status::not_found("model version not available"),
        _ => internal(),
    }
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
        let bytes = request.into_inner().model_bundle;
        let text = String::from_utf8(bytes)
            .map_err(|_| Status::invalid_argument("model must be UTF-8 YAML or JSON"))?;
        let issues = tokio::task::spawn_blocking(move || {
            if serde_json::from_str::<serde_json::Value>(&text).is_ok() {
                pp_cube_model::validate_json(&text)
            } else {
                pp_cube_model::validate_yaml(&text)
            }
        })
        .await
        .map_err(|_| internal())?;
        Ok(Response::new(ValidateModelResponse {
            issues: issues
                .into_iter()
                .map(|i| ModelIssue {
                    path: i.path,
                    rule: i.rule_id,
                    message: i.message,
                })
                .collect(),
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
