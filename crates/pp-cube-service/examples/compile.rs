//! A minimal client: compiles one query and prints the SQL and what to bind.
//!
//!   cargo run -p pp-cube-service --example compile -- \
//!     https://127.0.0.1:50051 v1 doris '{"measures":["sales.amount"]}' \
//!     [ca-certificate.pem] [token-file]
//!
//! With a CA certificate the connection is verified for the name `localhost`.

use pp_cube_service::grpc::wire::{
    compile_response, cube_engine_client::CubeEngineClient, parameter, CompileRequest,
};
use tonic::transport::{Certificate, Channel, ClientTlsConfig};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let [endpoint, version, dialect, query, rest @ ..] = arguments.as_slice() else {
        return Err(
            "usage: compile <endpoint> <version> <dialect> <query-json> [ca.pem] [token-file]"
                .into(),
        );
    };
    let mut channel = Channel::from_shared(endpoint.clone())?;
    if let Some(ca) = rest.first() {
        let tls = ClientTlsConfig::new()
            .ca_certificate(Certificate::from_pem(std::fs::read(ca)?))
            .domain_name("localhost");
        channel = channel.tls_config(tls)?;
    }
    let mut request = tonic::Request::new(CompileRequest {
        model_version: version.clone(),
        query_json: query.clone(),
        dialect: dialect.clone(),
        timezone: "UTC".into(),
        request_id: "example".into(),
        ..Default::default()
    });
    if let Some(path) = rest.get(1) {
        let token = std::fs::read_to_string(path)?;
        let value = format!("Bearer {}", token.trim()).parse()?;
        request.metadata_mut().insert("authorization", value);
    }
    let response = CubeEngineClient::new(channel.connect().await?)
        .compile(request)
        .await?
        .into_inner();
    match response.result {
        Some(compile_response::Result::Compiled(compiled)) => {
            println!("{}", compiled.sql);
            for (index, value) in compiled.params.iter().enumerate() {
                let value = match &value.value {
                    Some(parameter::Value::Text(text)) => format!("text {text:?}"),
                    Some(parameter::Value::Number(number)) => format!("number {number}"),
                    Some(parameter::Value::Boolean(flag)) => format!("boolean {flag}"),
                    _ => "null".to_owned(),
                };
                println!("-- parameter {}: {value}", index + 1);
            }
            for property in compiled.session_properties {
                println!("-- session: {}={}", property.name, property.value);
            }
        }
        Some(compile_response::Result::Failure(failure)) => {
            return Err(format!("compile failure: {}", failure.message).into())
        }
        None => return Err("empty response".into()),
    }
    Ok(())
}
