//! Host-mounted gRPC transport. The host chooses the listener and TLS; this layer
//! offers bearer-token access control, a compilation deadline, request telemetry
//! and the standard health protocol.
use crate::telemetry::{Event, Telemetry};
use crate::{CompilePool, CubeService, PoolError, ServiceError};
use pp_cube_compile::{CompileContext, CompileError, DialectId, Parameter as CompiledParameter};
use pp_cube_model::{load_query, BundleError};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tonic::{metadata::MetadataMap, Request, Response, Status};

pub mod wire {
    tonic::include_proto!("ppcube.engine.v1");
}
/// The standard `grpc.health.v1` protocol.
pub mod health {
    tonic::include_proto!("grpc.health.v1");
}

/// The name health probes may ask about, besides the empty name for the whole server.
pub const SERVICE_NAME: &str = "ppcube.engine.v1.CubeEngine";

/// Bearer tokens for callers. With no token configured every caller is trusted,
/// which is only acceptable on a socket the host controls.
#[derive(Clone, Default)]
pub struct Access {
    read: Option<Vec<u8>>,
    admin: Option<Vec<u8>>,
}
impl Access {
    /// `read` admits Compile, ValidateModel and GetMeta; `admin` admits everything.
    /// Tokens are at least 16 visible ASCII characters and must differ.
    pub fn new(read: Option<&str>, admin: Option<&str>) -> Result<Self, &'static str> {
        for token in [read, admin].into_iter().flatten() {
            if token.len() < 16 || !token.bytes().all(|b| b.is_ascii_graphic()) {
                return Err("a token is at least 16 visible ASCII characters");
            }
        }
        if read.is_some() && read == admin {
            return Err("the read and admin tokens must differ");
        }
        Ok(Self {
            read: read.map(|token| token.as_bytes().to_vec()),
            admin: admin.map(|token| token.as_bytes().to_vec()),
        })
    }

    pub fn requires_token(&self) -> bool {
        self.read.is_some() || self.admin.is_some()
    }
    pub fn has_admin_token(&self) -> bool {
        self.admin.is_some()
    }
}
#[derive(Clone, Copy, PartialEq)]
enum Role {
    Read,
    Admin,
}
/// Compares every byte whatever the first difference, so timing does not locate it.
fn same(a: &[u8], b: &[u8]) -> bool {
    let mut difference = a.len() ^ b.len();
    for index in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(index).copied(), b.get(index).copied());
        difference |= usize::from(x.unwrap_or(0) ^ y.unwrap_or(0));
    }
    difference == 0
}

/// Bounds a host sets on what one request may cost.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// How long a caller waits for a compilation. The planner cannot be
    /// interrupted, so a compilation that overruns still occupies its worker.
    pub compile_timeout: Duration,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            compile_timeout: Duration::from_secs(10),
        }
    }
}
use wire::*;

pub struct GrpcService {
    core: Arc<CubeService>,
    pool: CompilePool,
    management: bool,
    access: Access,
    limits: Limits,
    telemetry: Telemetry,
}
impl GrpcService {
    pub fn new(core: Arc<CubeService>, workers: usize, queue: usize) -> Result<Self, PoolError> {
        Ok(Self {
            pool: CompilePool::new(Arc::clone(&core), workers, queue)?,
            core,
            management: false,
            access: Access::default(),
            limits: Limits::default(),
            telemetry: Telemetry::default(),
        })
    }

    pub fn with_access(mut self, access: Access) -> Self {
        self.access = access;
        self
    }
    pub fn with_limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }
    pub fn with_telemetry(mut self, telemetry: Telemetry) -> Self {
        self.telemetry = telemetry;
        self
    }

    /// Who may call: nobody is asked for a token when none is configured.
    /// A valid read token on a management operation is a permission error;
    /// a missing or unknown token is an authentication error.
    fn authorize(&self, metadata: &MetadataMap, role: Role) -> Result<(), Status> {
        if !self.access.requires_token() || (role == Role::Read && self.access.read.is_none()) {
            return Ok(());
        }
        let presented = metadata
            .get("authorization")
            .and_then(|value| value.as_bytes().strip_prefix(b"Bearer "))
            .ok_or_else(|| Status::unauthenticated("a bearer token is required"))?;
        let is = |token: &Option<Vec<u8>>| token.as_deref().is_some_and(|t| same(t, presented));
        // Evaluate both so a wrong token costs the same whichever it resembles.
        let (read, admin) = (is(&self.access.read), is(&self.access.admin));
        match (role, read, admin) {
            (_, _, true) | (Role::Read, true, _) => Ok(()),
            (Role::Admin, true, false) => Err(Status::permission_denied(
                "this token does not allow model management",
            )),
            _ => Err(Status::unauthenticated("the bearer token is not valid")),
        }
    }

    /// Runs one operation between the access check and the telemetry record.
    async fn observed<T>(
        &self,
        mut event: Event,
        allowed: Result<(), Status>,
        operation: impl std::future::Future<Output = Result<Response<T>, Status>>,
        outcome: impl FnOnce(&T) -> Outcome,
    ) -> Result<Response<T>, Status> {
        let started = Instant::now();
        self.telemetry.metrics.started();
        let result = match allowed {
            Ok(()) => operation.await,
            Err(status) => Err(status),
        };
        event.duration = started.elapsed();
        event.outcome = match &result {
            Ok(response) => {
                let outcome = outcome(response.get_ref());
                event.fingerprint = outcome.fingerprint;
                outcome.name
            }
            Err(status) => format!("{:?}", status.code()),
        };
        self.telemetry.finish(event);
        result
    }

    /// Allow ListModels, PublishModel and RetireModel. Off by default: whoever may
    /// publish decides what SQL every caller gets, so the host must opt in and
    /// must control who can reach the service.
    pub fn with_management(mut self, enabled: bool) -> Self {
        self.management = enabled;
        self
    }

    fn manage(&self, metadata: &MetadataMap) -> Result<(), Status> {
        if !self.management {
            return Err(Status::permission_denied("model management is not enabled"));
        }
        // With tokens in use, management needs the admin token; a host that set
        // only a read token has not said who may publish.
        if self.access.requires_token() && !self.access.has_admin_token() {
            return Err(Status::permission_denied("no admin token is configured"));
        }
        self.authorize(metadata, Role::Admin)
    }
}
/// How an answered request ended, for telemetry.
struct Outcome {
    name: String,
    fingerprint: Option<String>,
}
fn ok() -> Outcome {
    Outcome {
        name: "ok".into(),
        fingerprint: None,
    }
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
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
        BundleError::CatalogFull => {
            Status::resource_exhausted("the limit on available model versions is reached")
        }
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
impl GrpcService {
    async fn compile_inner(
        &self,
        request: CompileRequest,
    ) -> Result<Response<CompileResponse>, Status> {
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
        let waited = tokio::time::timeout(self.limits.compile_timeout, task.wait()).await;
        let finished = waited.map_err(|_| {
            Status::deadline_exceeded("the compilation did not finish within the service deadline")
        })?;
        match finished {
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
    async fn validate_model_inner(
        &self,
        request: ValidateModelRequest,
    ) -> Result<Response<ValidateModelResponse>, Status> {
        let text = parse_model(request.model_bundle)?;
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
    async fn list_models_inner(&self) -> Result<Response<ListModelsResponse>, Status> {
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
    async fn publish_model_inner(
        &self,
        request: PublishModelRequest,
    ) -> Result<Response<PublishModelResponse>, Status> {
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
    async fn retire_model_inner(
        &self,
        request: RetireModelRequest,
    ) -> Result<Response<RetireModelResponse>, Status> {
        let bundle = self
            .core
            .retire(&request.model_version)
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
    async fn get_meta_inner(
        &self,
        request: GetMetaRequest,
    ) -> Result<Response<GetMetaResponse>, Status> {
        let core = Arc::clone(&self.core);
        let version = request.model_version;
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
fn event(rpc: &'static str, model_version: Option<&str>) -> Event {
    Event {
        rpc,
        model_version: model_version.map(str::to_owned),
        ..Event::default()
    }
}

#[tonic::async_trait]
impl cube_engine_server::CubeEngine for GrpcService {
    async fn compile(
        &self,
        request: Request<CompileRequest>,
    ) -> Result<Response<CompileResponse>, Status> {
        let allowed = self.authorize(request.metadata(), Role::Read);
        let request = request.into_inner();
        let event = Event {
            request_id: Some(request.request_id.clone()),
            // Only a known dialect name is logged; anything else is the caller's text.
            dialect: DialectId::from_name(&request.dialect).map(|d| d.name().to_owned()),
            ..event("Compile", Some(&request.model_version))
        };
        let outcome = |response: &CompileResponse| match &response.result {
            Some(compile_response::Result::Compiled(compiled)) => Outcome {
                name: "ok".into(),
                fingerprint: Some(hex(&compiled.fingerprint)),
            },
            Some(compile_response::Result::Failure(failure)) => Outcome {
                name: match FailureKind::try_from(failure.kind) {
                    Ok(FailureKind::Model) => "model",
                    Ok(FailureKind::Query) => "query",
                    Ok(FailureKind::Unsupported) => "unsupported",
                    _ => "internal",
                }
                .into(),
                fingerprint: None,
            },
            None => ok(),
        };
        self.observed(event, allowed, self.compile_inner(request), outcome)
            .await
    }
    async fn validate_model(
        &self,
        request: Request<ValidateModelRequest>,
    ) -> Result<Response<ValidateModelResponse>, Status> {
        let allowed = self.authorize(request.metadata(), Role::Read);
        let operation = self.validate_model_inner(request.into_inner());
        self.observed(event("ValidateModel", None), allowed, operation, |_| ok())
            .await
    }
    async fn get_meta(
        &self,
        request: Request<GetMetaRequest>,
    ) -> Result<Response<GetMetaResponse>, Status> {
        let allowed = self.authorize(request.metadata(), Role::Read);
        let request = request.into_inner();
        let event = event("GetMeta", Some(&request.model_version));
        self.observed(event, allowed, self.get_meta_inner(request), |_| ok())
            .await
    }
    async fn list_models(
        &self,
        request: Request<ListModelsRequest>,
    ) -> Result<Response<ListModelsResponse>, Status> {
        let allowed = self.manage(request.metadata());
        self.observed(
            event("ListModels", None),
            allowed,
            self.list_models_inner(),
            |_| ok(),
        )
        .await
    }
    async fn publish_model(
        &self,
        request: Request<PublishModelRequest>,
    ) -> Result<Response<PublishModelResponse>, Status> {
        let allowed = self.manage(request.metadata());
        let request = request.into_inner();
        let event = event("PublishModel", Some(&request.model_version));
        let outcome = |response: &PublishModelResponse| match response.result {
            Some(publish_model_response::Result::Rejected(_)) => Outcome {
                name: "model".into(),
                fingerprint: None,
            },
            _ => ok(),
        };
        self.observed(event, allowed, self.publish_model_inner(request), outcome)
            .await
    }
    async fn retire_model(
        &self,
        request: Request<RetireModelRequest>,
    ) -> Result<Response<RetireModelResponse>, Status> {
        let allowed = self.manage(request.metadata());
        let request = request.into_inner();
        let event = event("RetireModel", Some(&request.model_version));
        self.observed(event, allowed, self.retire_model_inner(request), |_| ok())
            .await
    }
}

/// The standard health service. It needs no token: a probe learns only whether
/// the server is accepting work.
#[derive(Clone)]
pub struct HealthService {
    serving: Arc<AtomicBool>,
}
impl HealthService {
    /// `serving` is shared with the host, which clears it when it starts to drain.
    pub fn new(serving: Arc<AtomicBool>) -> Self {
        Self { serving }
    }
}
#[tonic::async_trait]
impl health::health_server::Health for HealthService {
    async fn check(
        &self,
        request: Request<health::HealthCheckRequest>,
    ) -> Result<Response<health::HealthCheckResponse>, Status> {
        use health::health_check_response::ServingStatus;
        let service = request.into_inner().service;
        if !service.is_empty() && service != SERVICE_NAME {
            return Err(Status::not_found("unknown service"));
        }
        let status = if self.serving.load(Ordering::Relaxed) {
            ServingStatus::Serving
        } else {
            ServingStatus::NotServing
        };
        Ok(Response::new(health::HealthCheckResponse {
            status: status.into(),
        }))
    }
    type WatchStream = tokio_stream::Empty<Result<health::HealthCheckResponse, Status>>;
    /// Not offered: the protocol tells a client that receives UNIMPLEMENTED to
    /// stop watching and treat the server as healthy; probes use Check.
    async fn watch(
        &self,
        _request: Request<health::HealthCheckRequest>,
    ) -> Result<Response<Self::WatchStream>, Status> {
        Err(Status::unimplemented("use Check"))
    }
}

fn pool_status(error: PoolError) -> Status {
    match error {
        PoolError::Busy => Status::resource_exhausted("compilation queue is full"),
        PoolError::Unavailable => Status::unavailable("compilation worker unavailable"),
        _ => internal(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cube_engine_server::CubeEngine;
    use std::sync::mpsc;

    // scenario: B0-API-019
    #[tokio::test(flavor = "current_thread")]
    async fn a_compilation_that_overruns_the_deadline_is_reported_and_the_service_recovers() {
        let core = Arc::new(CubeService::new([(
            "warehouse".to_owned(),
            DialectId::Presto,
        )]));
        let source = "cubes:\n- name: sales\n  data_source: warehouse\n  sql_table: dw.sales\n  measures:\n  - {name: amount, type: sum, sql: '{CUBE}.amount'}\n";
        core.publish("v1", pp_cube_model::load_yaml(source).unwrap(), None)
            .unwrap();
        let telemetry = Telemetry::default();
        let service = GrpcService::new(core, 1, 4)
            .unwrap()
            .with_telemetry(telemetry.clone())
            .with_limits(Limits {
                compile_timeout: Duration::from_millis(50),
            });
        // Occupy the only worker, so the request is accepted but cannot start.
        let (started, running) = mpsc::channel();
        let (release, gate) = mpsc::channel::<()>();
        service
            .pool
            .enqueue(Box::new(move || {
                started.send(()).unwrap();
                let _ = gate.recv_timeout(Duration::from_secs(10));
            }))
            .unwrap();
        running.recv_timeout(Duration::from_secs(5)).unwrap();
        let request = || {
            Request::new(CompileRequest {
                model_version: "v1".into(),
                query_json: r#"{"measures":["sales.amount"]}"#.into(),
                dialect: "presto".into(),
                timezone: "UTC".into(),
                ..Default::default()
            })
        };
        let status = service.compile(request()).await.unwrap_err();
        assert_eq!(status.code(), tonic::Code::DeadlineExceeded);
        assert_eq!(telemetry.metrics.requests("Compile", "DeadlineExceeded"), 1);
        // Once the worker is free the abandoned compilation drains and requests succeed
        // again. How soon depends on the machine, so allow several deadlines.
        release.send(()).unwrap();
        let started = Instant::now();
        let recovered = loop {
            match service.compile(request()).await {
                Ok(response) => break response.into_inner(),
                Err(status) => {
                    assert_eq!(status.code(), tonic::Code::DeadlineExceeded);
                    assert!(started.elapsed() < Duration::from_secs(20), "no recovery");
                }
            }
        };
        assert!(matches!(
            recovered.result,
            Some(compile_response::Result::Compiled(_))
        ));
        assert_eq!(telemetry.metrics.requests("Compile", "ok"), 1);
    }
}
