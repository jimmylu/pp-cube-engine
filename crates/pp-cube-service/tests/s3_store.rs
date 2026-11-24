//! An S3-protocol bucket as a model store, against a stand-in server in this
//! process. Evidence against a real S3-protocol server is a separate tier.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use pp_cube_compile::DialectId;
use pp_cube_service::s3::{S3Config, S3Store};
use pp_cube_service::store::{ModelStore, StoreSync};
use pp_cube_service::CubeService;

const CUBE: &str = "cubes:\n- name: sales\n  sql_table: dw.sales\n  data_source: warehouse\n  measures:\n  - {name: amount, type: sum, sql: '{CUBE}.amount'}\n";
const BUCKET: &str = "semantic-assets";

/// Speaks just enough of the S3 protocol and of the instance metadata service:
/// a paginated ListObjectsV2, GetObject, and the three calls for a role.
struct StandIn {
    address: String,
    /// Method and target of every request, as received.
    requests: Arc<Mutex<Vec<String>>>,
    /// Answer every bucket request with this status instead.
    refuse: Arc<Mutex<Option<u16>>>,
}

fn respond(stream: &mut TcpStream, status: u16, body: &str) {
    let _ = write!(
        stream,
        "HTTP/1.1 {status} X\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
}

impl StandIn {
    fn start(objects: &[(&str, &str)]) -> Self {
        let objects: BTreeMap<String, String> = objects
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let refuse = Arc::new(Mutex::new(None));
        let (seen, refused) = (Arc::clone(&requests), Arc::clone(&refuse));
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut line = String::new();
                if reader.read_line(&mut line).is_err() {
                    continue;
                }
                let mut headers = String::new();
                let mut header = String::new();
                while reader.read_line(&mut header).is_ok() && header.trim() != "" {
                    headers.push_str(&header.to_ascii_lowercase());
                    header.clear();
                }
                let mut parts = line.split_whitespace();
                let (method, target) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
                seen.lock().unwrap().push(format!("{method} {target}"));
                let (path, query) = target.split_once('?').unwrap_or((target, ""));
                let role = "/latest/meta-data/iam/security-credentials/";
                if path == "/latest/api/token" && method == "PUT" {
                    respond(&mut stream, 200, "metadata-session");
                } else if let Some(name) = path.strip_prefix(role) {
                    // IMDSv2: no answer without the session token.
                    if !headers.contains("x-aws-ec2-metadata-token: metadata-session") {
                        respond(&mut stream, 401, "");
                    } else if name.is_empty() {
                        respond(&mut stream, 200, "replica-role\n");
                    } else {
                        respond(
                            &mut stream,
                            200,
                            r#"{"Code":"Success","Type":"AWS-HMAC","AccessKeyId":"ROLEKEYID","SecretAccessKey":"role-secret","Token":"role-session","Expiration":"2099-01-01T00:00:00Z"}"#,
                        );
                    }
                } else if let Some(status) = *refused.lock().unwrap() {
                    respond(
                        &mut stream,
                        status,
                        "<Error><Message>secret detail</Message></Error>",
                    );
                } else if !query.contains("X-Amz-Signature=") {
                    respond(&mut stream, 403, "");
                } else if path == format!("/{BUCKET}") || path == format!("/{BUCKET}/") {
                    // Two keys a page, to make the client ask again.
                    let prefix = query
                        .split('&')
                        .find_map(|pair| pair.strip_prefix("prefix="))
                        .unwrap_or("")
                        .replace("%2F", "/");
                    let keys: Vec<&String> =
                        objects.keys().filter(|k| k.starts_with(&prefix)).collect();
                    let start: usize = query
                        .split('&')
                        .find_map(|pair| pair.strip_prefix("continuation-token=page"))
                        .and_then(|page| page.parse().ok())
                        .unwrap_or(0);
                    let page: Vec<&&String> = keys.iter().skip(start * 2).take(2).collect();
                    let more = keys.len() > (start + 1) * 2;
                    let mut body = String::from(
                        r#"<?xml version="1.0" encoding="UTF-8"?><ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><Name>semantic-assets</Name><Prefix></Prefix><KeyCount>2</KeyCount><MaxKeys>2</MaxKeys>"#,
                    );
                    body.push_str(&format!("<IsTruncated>{more}</IsTruncated>"));
                    for key in page {
                        body.push_str(&format!("<Contents><Key>{key}</Key><LastModified>2026-11-24T00:00:00.000Z</LastModified><ETag>&quot;e&quot;</ETag><Size>1</Size><StorageClass>STANDARD</StorageClass></Contents>"));
                    }
                    if more {
                        body.push_str(&format!(
                            "<NextContinuationToken>page{}</NextContinuationToken>",
                            start + 1
                        ));
                    }
                    body.push_str("</ListBucketResult>");
                    respond(&mut stream, 200, &body);
                } else if let Some(text) = path
                    .strip_prefix(&format!("/{BUCKET}/"))
                    .and_then(|k| objects.get(k))
                {
                    respond(&mut stream, 200, text);
                } else {
                    respond(
                        &mut stream,
                        404,
                        "<Error><Message>secret detail</Message></Error>",
                    );
                }
                let _ = stream.flush();
                let _ = reader.read(&mut [0u8; 1]);
            }
        });
        Self {
            address,
            requests,
            refuse,
        }
    }

    fn config(&self, keys: bool) -> S3Config {
        S3Config {
            bucket: BUCKET.into(),
            prefix: "prod/".into(),
            region: "us-east-1".into(),
            endpoint: Some(self.address.clone()),
            keys: keys.then(|| ("STATICKEYID".into(), "static-secret".into(), None)),
            metadata_endpoint: self.address.clone(),
        }
    }

    fn seen(&self, part: &str) -> usize {
        let requests = self.requests.lock().unwrap();
        requests
            .iter()
            .filter(|request| request.contains(part))
            .count()
    }
}

fn objects() -> Vec<(&'static str, &'static str)> {
    vec![
        ("prod/property/motor/models/v1/cubes/sales.yml", CUBE),
        (
            "prod/property/motor/models/v1/manifest.json",
            r#"{"schema": 1}"#,
        ),
        ("prod/property/home/models/v1/cubes/sales.yml", CUBE),
        (
            "prod/property/home/models/v1/manifest.json",
            r#"{"schema": 1}"#,
        ),
        // A folder marker, a hidden entry, and keys of another prefix.
        ("prod/property/motor/models/v1/cubes/", ""),
        ("prod/property/motor/models/.staging/manifest.json", "x"),
        (
            "test/property/motor/models/v9/manifest.json",
            r#"{"schema": 1}"#,
        ),
    ]
}

// scenario: B1-STORE-004
#[test]
fn bucket_under_a_prefix_is_the_same_store_as_a_directory() {
    let server = StandIn::start(&objects());
    let store = S3Store::new(server.config(true)).unwrap();
    // Keys are relative to the prefix, over as many pages as the listing takes.
    assert_eq!(
        store.list().unwrap(),
        [
            "property/home/models/v1/cubes/sales.yml",
            "property/home/models/v1/manifest.json",
            "property/motor/models/v1/cubes/sales.yml",
            "property/motor/models/v1/manifest.json",
        ]
    );
    assert!(server.seen("continuation-token=page1") >= 1);
    assert_eq!(
        store
            .read("property/motor/models/v1/cubes/sales.yml")
            .unwrap(),
        CUBE
    );
    // Every request is signed with the configured key, for this bucket only.
    let requests = server.requests.lock().unwrap().clone();
    assert!(
        requests
            .iter()
            .all(|r| r.contains("X-Amz-Credential=STATICKEYID%2F")),
        "{requests:?}"
    );
    assert!(requests
        .iter()
        .all(|r| r.starts_with(&format!("GET /{BUCKET}"))));
    // A failure names what was asked and the status, never the server's text.
    let missing = store
        .read("property/motor/models/v1/cubes/none.yml")
        .unwrap_err();
    assert_eq!(missing.to_string(), "reading an object: status 404");
    for key in ["../outside", "property//x", ""] {
        assert!(store.read(key).is_err(), "{key}");
    }

    // The service offers its versions exactly as from a directory.
    let service = CubeService::new([("warehouse".to_owned(), DialectId::Presto)]);
    let sync = StoreSync::new(S3Store::new(server.config(true)).unwrap());
    let scan = sync.scan(&service).unwrap();
    assert_eq!(scan.loaded, ["property/home/v1", "property/motor/v1"]);
    assert!(scan.rejected.is_empty(), "{:?}", scan.rejected);
    let query = pp_cube_model::load_query(r#"{"measures": ["sales.amount"]}"#).unwrap();
    let context = pp_cube_compile::CompileContext {
        request_id: String::new(),
        dialect: DialectId::Presto,
        timezone: "UTC".into(),
        security_filters: vec![],
    };
    assert!(service
        .compile("property/motor/v1", &query, &context)
        .is_ok());

    // A bucket that refuses is a failed pass: nothing changes, and it is counted.
    *server.refuse.lock().unwrap() = Some(403);
    let error = sync.scan(&service).unwrap_err();
    assert_eq!(error.to_string(), "listing the bucket: status 403");
    assert_eq!(service.versions().len(), 2);
    assert!(sync
        .render()
        .contains("ppcube_model_store_last_scan_failed{store=\"0\"} 1"));
}

// scenario: B1-STORE-005
#[test]
fn credentials_come_from_the_environment_or_the_instance_role() {
    let env = |pairs: &[(&str, &str)]| {
        let pairs: BTreeMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        move |name: &str| pairs.get(name).cloned()
    };
    // The variables the AWS tools use; an endpoint makes it another S3-protocol store.
    let config = S3Config::from_env(
        "s3://semantic-assets/prod/",
        env(&[
            ("AWS_ACCESS_KEY_ID", "K"),
            ("AWS_SECRET_ACCESS_KEY", "S"),
            ("AWS_SESSION_TOKEN", "T"),
            ("AWS_REGION", "eu-west-1"),
            ("AWS_ENDPOINT_URL_S3", "https://objects.example"),
            ("AWS_ENDPOINT_URL", "https://other.example"),
        ]),
    )
    .unwrap();
    assert_eq!(
        (config.bucket.as_str(), config.prefix.as_str()),
        ("semantic-assets", "prod/")
    );
    assert_eq!(config.region, "eu-west-1");
    assert_eq!(config.endpoint.as_deref(), Some("https://objects.example"));
    assert_eq!(
        config.keys,
        Some(("K".into(), "S".into(), Some("T".into())))
    );
    // Nothing set: AWS itself, the default region, the instance role.
    let bare = S3Config::from_env("s3://semantic-assets", env(&[])).unwrap();
    assert_eq!(
        (bare.prefix.as_str(), bare.region.as_str()),
        ("", "us-east-1")
    );
    assert!(bare.keys.is_none() && bare.endpoint.is_none());
    assert_eq!(bare.metadata_endpoint, "http://169.254.169.254");
    for (location, pairs) in [
        ("semantic-assets/prod", vec![]),
        ("s3://", vec![]),
        ("s3://bucket/pre fix", vec![]),
        ("s3://bucket", vec![("AWS_ACCESS_KEY_ID", "K")]),
        ("s3://bucket", vec![("AWS_SECRET_ACCESS_KEY", "S")]),
        // Not silently taken for the instance role.
        (
            "s3://bucket",
            vec![("AWS_WEB_IDENTITY_TOKEN_FILE", "/var/run/token")],
        ),
    ] {
        assert!(
            S3Config::from_env(location, env(&pairs)).is_err(),
            "{location} {pairs:?}"
        );
    }

    // Without keys the store asks the metadata service, with a session token, and
    // signs with what the role gives, session token included.
    let server = StandIn::start(&objects());
    let store = S3Store::new(server.config(false)).unwrap();
    assert_eq!(store.list().unwrap().len(), 4);
    assert_eq!(server.seen("PUT /latest/api/token"), 1);
    assert_eq!(server.seen("security-credentials/replica-role"), 1);
    let signed = server.requests.lock().unwrap().clone();
    let bucket: Vec<_> = signed.iter().filter(|r| r.contains(BUCKET)).collect();
    assert!(!bucket.is_empty());
    assert!(bucket
        .iter()
        .all(|r| r.contains("X-Amz-Credential=ROLEKEYID%2F")
            && r.contains("X-Amz-Security-Token=role-session")));
    // Kept for a while: more requests ask the metadata service nothing more.
    store.list().unwrap();
    store
        .read("property/motor/models/v1/manifest.json")
        .unwrap();
    assert_eq!(server.seen("PUT /latest/api/token"), 1);
}

// scenario: B1-API-003
#[test]
fn a_replica_reads_a_directory_and_a_bucket_at_once() {
    let server = StandIn::start(&objects());
    let root = std::env::temp_dir().join(format!("pp-s3-both-{}", std::process::id()));
    let version = root.join("liability/models/v1");
    std::fs::create_dir_all(version.join("cubes")).unwrap();
    std::fs::write(version.join("cubes/sales.yml"), CUBE).unwrap();
    std::fs::write(version.join("manifest.json"), r#"{"schema": 1}"#).unwrap();
    let command = || {
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_pp-cube-service"));
        command
            .env_remove("AWS_SESSION_TOKEN")
            .env_remove("AWS_WEB_IDENTITY_TOKEN_FILE")
            .env("AWS_ACCESS_KEY_ID", "STATICKEYID")
            .env("AWS_SECRET_ACCESS_KEY", "static-secret")
            .env("AWS_ENDPOINT_URL_S3", &server.address)
            .args([
                "--listen",
                "127.0.0.1:0",
                "--data-source",
                "warehouse=presto",
            ]);
        command
    };
    // A bucket that cannot be listed stops the process, as a missing directory does.
    let output = command()
        .args(["--model-store", "s3://no-such-bucket/prod"])
        .output()
        .unwrap();
    assert!(!output.status.success() && output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("listing the bucket: status 4"));
    let output = command()
        .args(["--model-store", "gs://bucket"])
        .output()
        .unwrap();
    assert!(!output.status.success() && output.stdout.is_empty());

    let mut child = command()
        .args(["--model-store", root.to_str().unwrap()])
        .args(["--model-store", &format!("s3://{BUCKET}/prod")])
        .args(["--metrics-listen", "127.0.0.1:0"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let listening = lines.next().unwrap().unwrap();
    let metrics = lines.next().unwrap().unwrap();
    assert!(listening.starts_with("listening "));
    // Both stores were read before the address was announced, each under its number.
    let mut stream = TcpStream::connect(metrics.strip_prefix("metrics ").unwrap()).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    stream.write_all(b"GET /metrics HTTP/1.1\r\n\r\n").unwrap();
    let mut body = String::new();
    let _ = stream.read_to_string(&mut body);
    child.kill().unwrap();
    let mut log = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut log)
        .unwrap();
    let _ = child.wait();
    for line in [
        "ppcube_model_versions 3",
        "ppcube_model_store_scans_total{store=\"0\",outcome=\"ok\"} 1",
        "ppcube_model_store_scans_total{store=\"1\",outcome=\"ok\"} 1",
    ] {
        assert!(body.contains(line), "{line}\n{body}");
    }
    for version in ["liability/v1", "property/home/v1", "property/motor/v1"] {
        assert!(
            log.contains(&format!(
                r#""model_version":"{version}","outcome":"loaded""#
            )),
            "{log}"
        );
    }
    // No secret reaches the log.
    assert!(!log.contains("static-secret") && !log.contains("STATICKEYID"));
    let _ = std::fs::remove_dir_all(root);
}
