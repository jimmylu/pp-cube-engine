//! Host-mounted gRPC transport for trusted callers; no listener or authentication defaults.
use crate::{CompilePool, CubeService, PoolError, ServiceError};
use pp_cube_compile::{CompileContext, CompileError, DialectId, Parameter};
use pp_cube_model::{load_query, BundleError};
use std::{collections::BTreeSet, sync::Arc};
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
fn failure(kind: &str, message: String) -> Response<CompileResponse> {
    Response::new(CompileResponse {
        result: Some(compile_response::Result::Failure(CompileFailure {
            kind: kind.into(),
            message,
        })),
    })
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
        let dialect = match request.dialect.as_str() {
            "presto" => DialectId::Presto,
            "doris" | "druid" => {
                return Ok(failure("unsupported", "dialect is not implemented".into()))
            }
            _ => return Err(Status::invalid_argument("unknown dialect")),
        };
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
                    .map(|p| match p {
                        Parameter::Text(v) => serde_json::to_string(&v).map_err(|_| internal()),
                        Parameter::Bool(v) => Ok(v.to_string()),
                        Parameter::Null => Ok("null".into()),
                        Parameter::Number(v) if v.is_finite() => Ok(v.to_string()),
                        Parameter::Number(_) => Err(internal()),
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let columns = result
                    .columns
                    .into_iter()
                    .map(|c| {
                        Ok(OutputColumn {
                            alias: c.alias,
                            member: c.member,
                            r#type: serde_json::to_value(c.semantic_type)
                                .map_err(|_| internal())?
                                .as_str()
                                .ok_or_else(internal)?
                                .to_owned(),
                            granularity: c.granularity,
                        })
                    })
                    .collect::<Result<Vec<_>, Status>>()?;
                let used_tables: BTreeSet<_> = result
                    .used_cubes
                    .into_iter()
                    .flat_map(|c| c.physical_tables.into_iter().map(|p| p.join(".")))
                    .collect();
                Ok(Response::new(CompileResponse {
                    result: Some(compile_response::Result::Compiled(Compiled {
                        sql: result.sql,
                        params,
                        columns,
                        used_tables: used_tables.into_iter().collect(),
                        fingerprint: result.fingerprint.to_vec(),
                    })),
                }))
            }
            Err(PoolError::Compile(ServiceError::Compile(error))) => {
                let kind = match &error {
                    CompileError::Model(_) => "model",
                    CompileError::Query(_) => "query",
                    CompileError::Unsupported(_) => "unsupported",
                    CompileError::Internal { .. } => "internal",
                };
                Ok(failure(kind, error.to_string()))
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
        let issues = tokio::task::spawn_blocking(move || pp_cube_model::validate_yaml(&text))
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
        let meta = tokio::task::spawn_blocking(move || core.get_meta(&version))
            .await
            .map_err(|_| internal())?
            .map_err(bundle_error)?;
        Ok(Response::new(GetMetaResponse {
            meta_json: serde_json::to_string(&meta).map_err(|_| internal())?,
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
