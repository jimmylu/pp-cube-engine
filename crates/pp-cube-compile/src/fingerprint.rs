//! Versioned identity of typed compile inputs, independent of correlation ids.
use crate::{CompileContext, CompileError};
use pp_cube_model::{Model, Query};
use sha2::{Digest, Sha256};

// Update with vendor upgrades; the repository-only test checks the gitlink.
/// The upstream planner commit this kernel is built against.
pub const UPSTREAM_PIN: &str = "486f0d26bab58ab4fe0be0e4cdd39818b35a7870";

/// The digest of a model's typed content: `sha256:` and 64 hex digits. It does not
/// depend on YAML or JSON formatting, key order or comments, only on what loads.
pub fn model_digest(model: &Model) -> Result<String, CompileError> {
    let model = canonical(serde_json::to_value(model).map_err(error)?).map_err(error)?;
    let digest = Sha256::digest(model);
    Ok(digest.iter().fold("sha256:".to_owned(), |mut text, byte| {
        text.push_str(&format!("{byte:02x}"));
        text
    }))
}

pub(crate) fn fingerprint(
    model: &Model,
    query: &Query,
    ctx: &CompileContext,
) -> Result<[u8; 32], CompileError> {
    identity(model, query, ctx, crate::KERNEL_VERSION, UPSTREAM_PIN)
}
fn identity(
    model: &Model,
    query: &Query,
    ctx: &CompileContext,
    version: &str,
    pin: &str,
) -> Result<[u8; 32], CompileError> {
    let model = serde_json::to_value(model).map_err(error)?;
    let model = canonical(model).map_err(error)?;
    let digest = Sha256::digest(model);
    let record = serde_json::json!([
        "pp-cube-fingerprint-v1",
        digest.as_slice(),
        query,
        ctx.dialect.name(),
        ctx.timezone,
        ctx.security_filters,
        version,
        pin
    ]);
    let encoded = canonical(record).map_err(error)?;
    Ok(Sha256::digest(encoded).into())
}
fn error(e: serde_json::Error) -> CompileError {
    CompileError::Internal {
        detail: e.to_string(),
    }
}

fn canonical(mut value: serde_json::Value) -> Result<Vec<u8>, serde_json::Error> {
    fn sort(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(m) => {
                m.sort_keys();
                for v in m.values_mut() {
                    sort(v);
                }
            }
            serde_json::Value::Array(a) => {
                for v in a {
                    sort(v);
                }
            }
            _ => {}
        }
    }
    sort(&mut value);
    serde_json::to_vec(&value)
}
// scenario: B0-DET-002
#[test]
fn version_and_pin_participate_in_identity() {
    let model = pp_cube_model::load_yaml(include_str!("../tests/fixtures/sales.yaml")).unwrap();
    let query = pp_cube_model::Query::default();
    let ctx = CompileContext {
        dialect: crate::DialectId::Presto,
        timezone: "UTC".into(),
        security_filters: vec![],
        request_id: String::new(),
    };
    let base = identity(&model, &query, &ctx, "v1", "pin1").unwrap();
    assert_ne!(base, identity(&model, &query, &ctx, "v2", "pin1").unwrap());
    assert_ne!(base, identity(&model, &query, &ctx, "v1", "pin2").unwrap());
}
// scenario: B0-DET-002
#[test]
fn canonical_objects_preserve_numbers_and_array_order() {
    let first = serde_json::json!({"z":{"b":18446744073709551615u64,"a":1.25},"a":[1,2]});
    let second = serde_json::json!({"a":[1,2],"z":{"a":1.25,"b":18446744073709551615u64}});
    assert_eq!(
        canonical(first.clone()).unwrap(),
        canonical(second).unwrap()
    );
    assert!(String::from_utf8(canonical(first.clone()).unwrap())
        .unwrap()
        .contains("18446744073709551615"));
    let mut reversed = first;
    reversed["a"] = serde_json::json!([2, 1]);
    assert_ne!(
        canonical(reversed).unwrap(),
        canonical(serde_json::json!({"a":[1,2],"z":{"a":1.25,"b":18446744073709551615u64}}))
            .unwrap()
    );
}
// scenario: B0-DET-002
// scenario: B0-UPSTREAM-001
#[test]
fn upstream_identity_matches_repository_gitlink() {
    let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    // Source archives have no gitlink to validate. A worktree's .git is a file.
    if !root.join(".git").exists() {
        eprintln!("gitlink check skipped: source archive without Git metadata");
        return;
    }
    let output = match std::process::Command::new("git")
        .args(["ls-tree", "HEAD", "vendor/cube"])
        .current_dir(root)
        .output()
    {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("gitlink check skipped: Git executable unavailable");
            return;
        }
        Err(error) => panic!("cannot run Git gitlink check: {error}"),
    };
    assert!(
        output.status.success(),
        "git ls-tree failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout.trim(),
        format!("160000 commit {UPSTREAM_PIN}\tvendor/cube")
    );
    let checkout = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root.join("vendor/cube"))
        .output()
        .expect("read the checked-out upstream identity");
    assert!(
        checkout.status.success(),
        "cannot read upstream checkout: {}",
        String::from_utf8_lossy(&checkout.stderr)
    );
    let checkout = String::from_utf8(checkout.stdout).unwrap();
    let checkout = checkout.trim();
    assert_eq!(checkout, UPSTREAM_PIN);

    let model = std::sync::Arc::new(
        pp_cube_model::load_yaml(include_str!("../tests/fixtures/sales.yaml")).unwrap(),
    );
    let query = pp_cube_model::load_query(r#"{"measures":["sales.amount"]}"#).unwrap();
    let ctx = CompileContext {
        dialect: crate::DialectId::Presto,
        timezone: "UTC".into(),
        security_filters: vec![],
        request_id: String::new(),
    };
    let compiled = crate::compile(&model, &query, &ctx).unwrap();
    assert_eq!(
        compiled.fingerprint,
        identity(&model, &query, &ctx, env!("CARGO_PKG_VERSION"), checkout).unwrap()
    );
    assert_ne!(
        compiled.fingerprint,
        identity(
            &model,
            &query,
            &ctx,
            env!("CARGO_PKG_VERSION"),
            "different-upstream"
        )
        .unwrap()
    );
}
