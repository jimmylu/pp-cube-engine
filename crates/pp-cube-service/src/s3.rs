//! An S3-protocol bucket as a model store: the same keys a directory holds,
//! under a prefix of the bucket.
//!
//! Requests are signed by `rusty-s3` and sent by a small blocking client. The
//! service holds no secret of its own: credentials come from the standard
//! environment variables or, without them, from the instance role.

use std::io::{Error, Result};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use rusty_s3::actions::{ListObjectsV2, S3Action};
use rusty_s3::credentials::Ec2SecurityCredentialsMetadataResponse;
use rusty_s3::{Bucket, Credentials, UrlStyle};

use crate::store::{ModelStore, MAX_FILE_BYTES, MAX_KEYS};

/// How long a signed request stays valid, and how long any request may take.
const SIGNED_FOR: Duration = Duration::from_secs(60);
const TIMEOUT: Duration = Duration::from_secs(30);
/// Credentials of an instance role are asked for again after this long; the
/// metadata service renews them well before they expire.
const ROLE_REFRESH: Duration = Duration::from_secs(300);

/// Where a bucket is and how to sign for it. Built from `s3://bucket/prefix`
/// and the environment; nothing here is read from model content.
#[derive(Clone)]
pub struct S3Config {
    pub bucket: String,
    /// Keys of the store lie under this prefix; empty, or ending in `/`.
    pub prefix: String,
    pub region: String,
    /// A custom endpoint, for S3-protocol storage other than AWS. Addressed
    /// path-style; AWS itself is addressed by bucket host name.
    pub endpoint: Option<String>,
    /// Access key, secret and optional session token; `None` uses the instance role.
    pub keys: Option<(String, String, Option<String>)>,
    /// The instance metadata service.
    pub metadata_endpoint: String,
}

impl S3Config {
    /// Reads `s3://bucket[/prefix]` with the variables the AWS tools use:
    /// `AWS_REGION` or `AWS_DEFAULT_REGION`, `AWS_ENDPOINT_URL_S3` or
    /// `AWS_ENDPOINT_URL`, `AWS_ACCESS_KEY_ID` with `AWS_SECRET_ACCESS_KEY` and
    /// optionally `AWS_SESSION_TOKEN`, and `AWS_EC2_METADATA_SERVICE_ENDPOINT`.
    pub fn from_env(
        location: &str,
        variable: impl Fn(&str) -> Option<String>,
    ) -> std::result::Result<Self, String> {
        let variable = |name: &str| variable(name).filter(|value| !value.trim().is_empty());
        let rest = location
            .strip_prefix("s3://")
            .ok_or("an S3 store is written s3://bucket/prefix")?;
        let (bucket, prefix) = rest.split_once('/').unwrap_or((rest, ""));
        let named = |name: &str| {
            !name.is_empty()
                && name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_'))
        };
        let prefix = prefix.trim_matches('/');
        if !named(bucket)
            || !prefix
                .split('/')
                .all(|part| prefix.is_empty() || named(part))
        {
            return Err("an S3 store is written s3://bucket/prefix".into());
        }
        let keys = match (
            variable("AWS_ACCESS_KEY_ID"),
            variable("AWS_SECRET_ACCESS_KEY"),
        ) {
            (Some(key), Some(secret)) => Some((key, secret, variable("AWS_SESSION_TOKEN"))),
            (None, None) => None,
            _ => return Err("AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY go together".into()),
        };
        // Named so that it is not mistaken for the instance role.
        if keys.is_none() && variable("AWS_WEB_IDENTITY_TOKEN_FILE").is_some() {
            return Err("web identity credentials are not supported; give access keys".into());
        }
        Ok(Self {
            bucket: bucket.to_owned(),
            prefix: if prefix.is_empty() {
                String::new()
            } else {
                format!("{prefix}/")
            },
            region: variable("AWS_REGION")
                .or_else(|| variable("AWS_DEFAULT_REGION"))
                .unwrap_or_else(|| "us-east-1".to_owned()),
            endpoint: variable("AWS_ENDPOINT_URL_S3").or_else(|| variable("AWS_ENDPOINT_URL")),
            keys,
            metadata_endpoint: variable("AWS_EC2_METADATA_SERVICE_ENDPOINT")
                .unwrap_or_else(|| "http://169.254.169.254".to_owned()),
        })
    }
}

pub struct S3Store {
    bucket: Bucket,
    prefix: String,
    keys: Option<Credentials>,
    metadata_endpoint: String,
    role: Mutex<Option<(Instant, Credentials)>>,
    agent: ureq::Agent,
}

/// A failure that names what was asked and how it ended, never a response body.
fn failed(what: &str, error: &ureq::Error) -> Error {
    let ended = match error {
        ureq::Error::StatusCode(code) => format!("status {code}"),
        ureq::Error::Timeout(_) => "timed out".to_owned(),
        _ => "no response".to_owned(),
    };
    Error::other(format!("{what}: {ended}"))
}

impl S3Store {
    pub fn new(config: S3Config) -> std::result::Result<Self, String> {
        let (endpoint, style) = match &config.endpoint {
            Some(endpoint) => (endpoint.clone(), UrlStyle::Path),
            None => (
                format!("https://s3.{}.amazonaws.com", config.region),
                UrlStyle::VirtualHost,
            ),
        };
        let endpoint = endpoint
            .parse()
            .map_err(|_| "the S3 endpoint is not a URL")?;
        let bucket = Bucket::new(endpoint, style, config.bucket, config.region)
            .map_err(|_| "the S3 endpoint is not an http or https URL with a host")?;
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            .max_redirects(0)
            .build();
        Ok(Self {
            bucket,
            prefix: config.prefix,
            keys: config.keys.map(|(key, secret, token)| match token {
                Some(token) => Credentials::new_with_token(key, secret, token),
                None => Credentials::new(key, secret),
            }),
            metadata_endpoint: config.metadata_endpoint.trim_end_matches('/').to_owned(),
            role: Mutex::new(None),
            agent: agent.into(),
        })
    }

    /// The keys to sign with: the configured ones, or the instance role's, asked
    /// of the metadata service with a session token (IMDSv2) and kept for a while.
    fn credentials(&self) -> Result<Credentials> {
        if let Some(keys) = &self.keys {
            return Ok(keys.clone());
        }
        let mut role = self.role.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((taken, credentials)) = role.as_ref() {
            if taken.elapsed() < ROLE_REFRESH {
                return Ok(credentials.clone());
            }
        }
        let what = "instance role credentials";
        let text =
            |response: std::result::Result<ureq::http::Response<ureq::Body>, ureq::Error>| {
                let mut response = response.map_err(|error| failed(what, &error))?;
                let body = response.body_mut().with_config().limit(64 * 1024);
                body.read_to_string().map_err(|error| failed(what, &error))
            };
        let base = format!("{}/latest", self.metadata_endpoint);
        let token = text(
            self.agent
                .put(format!("{base}/api/token"))
                .header("x-aws-ec2-metadata-token-ttl-seconds", "21600")
                .send_empty(),
        )?;
        let get = |path: &str| {
            let request = self
                .agent
                .get(format!("{base}/meta-data/iam/security-credentials/{path}"));
            text(
                request
                    .header("x-aws-ec2-metadata-token", token.trim())
                    .call(),
            )
        };
        let name = get("")?;
        let name = name.lines().next().unwrap_or_default().trim();
        if name.is_empty() || name.contains('/') {
            return Err(Error::other(format!("{what}: no role")));
        }
        let credentials = Ec2SecurityCredentialsMetadataResponse::deserialize(&get(name)?)
            .map_err(|_| Error::other(format!("{what}: not understood")))?
            .into_credentials();
        *role = Some((Instant::now(), credentials.clone()));
        Ok(credentials)
    }
}

impl ModelStore for S3Store {
    fn list(&self) -> Result<Vec<String>> {
        let credentials = self.credentials()?;
        let mut keys = Vec::new();
        let mut next: Option<String> = None;
        loop {
            let mut action = ListObjectsV2::new(&self.bucket, Some(&credentials));
            if !self.prefix.is_empty() {
                action.with_prefix(self.prefix.as_str());
            }
            if let Some(token) = &next {
                action.with_continuation_token(token.as_str());
            }
            let what = "listing the bucket";
            let mut response = self
                .agent
                .get(action.sign(SIGNED_FOR).as_str())
                .call()
                .map_err(|error| failed(what, &error))?;
            let body = response.body_mut().with_config().limit(64 * 1024 * 1024);
            let text = body
                .read_to_string()
                .map_err(|error| failed(what, &error))?;
            let page = ListObjectsV2::parse_response(&text)
                .map_err(|_| Error::other(format!("{what}: not understood")))?;
            for object in page.contents {
                let Some(key) = object.key.strip_prefix(&self.prefix) else {
                    continue;
                };
                // Folder markers and hidden entries are not part of the store.
                if key.is_empty()
                    || key
                        .split('/')
                        .any(|part| part.is_empty() || part.starts_with('.'))
                {
                    continue;
                }
                if keys.len() >= MAX_KEYS {
                    return Err(Error::other("the store holds too many files"));
                }
                keys.push(key.to_owned());
            }
            next = page.next_continuation_token;
            if next.is_none() {
                break;
            }
        }
        keys.sort();
        Ok(keys)
    }

    fn read(&self, key: &str) -> Result<String> {
        if key
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(Error::other("not a key of the store"));
        }
        let credentials = self.credentials()?;
        let object = format!("{}{key}", self.prefix);
        let action = self.bucket.get_object(Some(&credentials), &object);
        let what = "reading an object";
        let mut response = self
            .agent
            .get(action.sign(SIGNED_FOR).as_str())
            .call()
            .map_err(|error| failed(what, &error))?;
        // One byte over the limit is read, so a file of the limit's size passes.
        let body = response.body_mut().with_config().limit(MAX_FILE_BYTES);
        body.read_to_string().map_err(|error| match error {
            ureq::Error::BodyExceedsLimit(_) => Error::other("the file is too large"),
            other => failed(what, &other),
        })
    }
}
