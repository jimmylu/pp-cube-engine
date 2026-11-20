//! Accumulated structural diagnostics. SQL semantics are not certified here.
use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;
use serde_norway::Value;

use crate::{locate_references, Model, Reference, ReferenceError};

/// A structural fault. Paths are JSON Pointers; the empty path is the root.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ValidationIssue {
    pub path: String,
    /// Stable machine-readable rule, independent of the human message.
    pub rule_id: String,
    pub message: String,
}

fn issue(out: &mut Vec<ValidationIssue>, path: &str, rule: &str, message: &str) {
    out.push(ValidationIssue {
        path: path.into(),
        rule_id: rule.into(),
        message: message.into(),
    });
}
fn path(base: &str, key: &str) -> String {
    format!("{base}/{}", key.replace('~', "~0").replace('/', "~1"))
}
fn syntax() -> Vec<ValidationIssue> {
    let mut out = Vec::new();
    issue(&mut out, "", "syntax", "The document could not be parsed");
    out
}
/// Validates YAML structure; an empty report does not certify SQL constraints.
pub fn validate_yaml(text: &str) -> Vec<ValidationIssue> {
    match serde_norway::from_str(text) {
        Ok(value) => validate(value),
        Err(_) => syntax(),
    }
}
/// Validates strict JSON structure, retaining the loader's parser settings.
pub fn validate_json(text: &str) -> Vec<ValidationIssue> {
    let mut parser = serde_json_lenient::Deserializer::from_str(text);
    parser.set_ignore_trailing_commas(false);
    parser.set_allow_comments(false);
    match Value::deserialize(&mut parser).and_then(|v| parser.end().map(|()| v)) {
        Ok(value) => validate(value),
        Err(_) => syntax(),
    }
}
/// Checks typed definitions, including programmatic mutations. Unknown fields
/// cannot survive deserialization. Deferred SQL restrictions are not checked.
pub fn validate_model(model: &Model) -> Vec<ValidationIssue> {
    match serde_norway::to_value(model) {
        Ok(value) => validate(value),
        Err(_) => syntax(),
    }
}

fn text<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key)?.as_str()
}
fn items<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v.get(key)
        .and_then(Value::as_sequence)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn validate(value: Value) -> Vec<ValidationIssue> {
    let mut out = Vec::new();
    shape(&value, "", "model", &mut out);
    semantics(&value, &mut out);
    out.sort();
    out.dedup();
    out
}

// Required fields, optional fields, nested collection element kinds.
fn fields(kind: &str) -> (&str, &str, &[(&str, &str)]) {
    match kind {
        "model" => ("cubes", "", &[("cubes", "cube")]),
        "cube" => ("name data_source", "sql_table sql sql_alias title description public meta joins dimensions measures segments", &[("joins","join"),("dimensions","dimension"),("measures","measure"),("segments","segment")]),
        "dimension" => ("name sql type", "primary_key title description public meta", &[]),
        "measure" => ("name type", "sql filters title description public meta format", &[("filters","filter")]),
        "join" => ("name relationship sql", "", &[]),
        "segment" => ("name sql", "", &[]),
        _ => ("sql", "", &[]),
    }
}
fn shape(v: &Value, at: &str, kind: &str, out: &mut Vec<ValidationIssue>) {
    let Some(map) = v.as_mapping() else {
        issue(out, at, "type", "Expected an object");
        return;
    };
    let (required, optional, collections) = fields(kind);
    for key in required.split_whitespace() {
        if v.get(key).is_none() {
            issue(out, &path(at, key), "required", "Required field is absent");
        }
    }
    for (key, val) in map {
        let Some(key) = key.as_str() else {
            issue(out, at, "type", "Object keys must be strings");
            continue;
        };
        let here = path(at, key);
        if !required
            .split_whitespace()
            .chain(optional.split_whitespace())
            .any(|f| f == key)
        {
            issue(
                out,
                &here,
                "unknown-field",
                "Field is outside the model contract",
            );
            continue;
        }
        if let Some((_, child)) = collections.iter().find(|(field, _)| *field == key) {
            if let Some(seq) = val.as_sequence() {
                for (i, entry) in seq.iter().enumerate() {
                    shape(entry, &path(&here, &i.to_string()), child, out);
                }
            } else {
                issue(out, &here, "type", "Expected a list");
            }
        } else if key == "meta" {
            if crate::meta::deserialize(val.clone()).is_err() {
                issue(out, &here, "type", "Metadata cannot be preserved exactly");
            }
        } else if val.is_null()
            && optional.split_whitespace().any(|f| f == key)
            && key != "primary_key"
        {
            continue;
        } else {
            let valid = match key {
                "public" | "primary_key" => val.as_bool().is_some(),
                "type" if kind == "dimension" => val
                    .as_str()
                    .is_some_and(|s| ["string", "number", "boolean", "time"].contains(&s)),
                "type" => val.as_str().is_some_and(|s| {
                    [
                        "count",
                        "count_distinct",
                        "sum",
                        "avg",
                        "min",
                        "max",
                        "number",
                    ]
                    .contains(&s)
                }),
                "relationship" => val
                    .as_str()
                    .is_some_and(|s| ["many_to_one", "one_to_one"].contains(&s)),
                _ => val.as_str().is_some(),
            };
            if !valid {
                issue(out, &here, "type", "Field has an invalid type or value");
            }
        }
    }
}
fn semantics(value: &Value, out: &mut Vec<ValidationIssue>) {
    let cubes = items(value, "cubes");
    let mut names = BTreeMap::new();
    let mut ambiguous = BTreeSet::new();
    for (i, cube) in cubes.iter().enumerate() {
        if let Some(name) = text(cube, "name") {
            if names.insert(name, i).is_some() {
                ambiguous.insert(name);
                issue(
                    out,
                    &format!("/cubes/{i}/name"),
                    "duplicate-name",
                    "Cube name is repeated",
                );
            }
        }
    }
    let mut pairs = BTreeSet::new();
    for (i, cube) in cubes.iter().enumerate() {
        if !cube.is_mapping() {
            continue;
        }
        let base = format!("/cubes/{i}");
        let sources: Vec<_> = ["sql_table", "sql"]
            .into_iter()
            .filter(|s| cube.get(*s).is_some_and(|v| !v.is_null()))
            .collect();
        if sources.len() != 1
            || sources
                .iter()
                .any(|s| text(cube, s).is_some_and(|s| s.trim().is_empty()))
        {
            issue(
                out,
                &base,
                "source",
                "Exactly one nonempty sql_table or sql is required",
            );
        }
        let mut members = BTreeSet::new();
        for group in ["dimensions", "measures", "segments"] {
            for (j, member) in items(cube, group).iter().enumerate() {
                let at = format!("{base}/{group}/{j}");
                if let Some(name) = text(member, "name") {
                    if !members.insert(name) {
                        issue(
                            out,
                            &path(&at, "name"),
                            "duplicate-name",
                            "Member name is repeated",
                        );
                    }
                }
                if group == "measures"
                    && text(member, "type").is_some_and(|t| t != "count")
                    && member.get("sql").is_none_or(Value::is_null)
                {
                    issue(
                        out,
                        &path(&at, "sql"),
                        "required",
                        "Only count may omit sql",
                    );
                }
            }
        }
        for group in ["dimensions", "measures", "segments", "joins"] {
            for (j, member) in items(cube, group).iter().enumerate() {
                let at = format!("{base}/{group}/{j}");
                check_expression(
                    member,
                    cube,
                    &path(&at, "sql"),
                    &names,
                    &ambiguous,
                    cubes,
                    out,
                );
                if group == "measures" {
                    for (k, filter) in items(member, "filters").iter().enumerate() {
                        check_expression(
                            filter,
                            cube,
                            &format!("{at}/filters/{k}/sql"),
                            &names,
                            &ambiguous,
                            cubes,
                            out,
                        );
                    }
                }
                if group != "joins" {
                    continue;
                }
                let Some(target) = text(member, "name") else {
                    continue;
                };
                if ambiguous.contains(target) {
                    continue;
                }
                let Some(&target_index) = names.get(target) else {
                    issue(
                        out,
                        &path(&at, "name"),
                        "join-target",
                        "Join target is not declared",
                    );
                    continue;
                };
                let pair = (i.min(target_index), i.max(target_index));
                if !pairs.insert(pair) {
                    issue(
                        out,
                        &at,
                        "duplicate-join",
                        "Only one join per cube pair is allowed",
                    );
                }
                for endpoint in [i, target_index] {
                    // An unusable dimension list already has a shape issue.
                    if cubes[endpoint]
                        .get("dimensions")
                        .is_some_and(|v| !v.is_sequence())
                    {
                        continue;
                    }
                    if !items(&cubes[endpoint], "dimensions")
                        .iter()
                        .any(|d| d.get("primary_key").and_then(Value::as_bool) == Some(true))
                    {
                        issue(
                            out,
                            &at,
                            "join-primary-key",
                            &format!("Cube at /cubes/{endpoint} needs a primary key"),
                        );
                    }
                }
            }
        }
    }
}
fn check_expression(
    v: &Value,
    own: &Value,
    at: &str,
    names: &BTreeMap<&str, usize>,
    ambiguous: &BTreeSet<&str>,
    cubes: &[Value],
    out: &mut Vec<ValidationIssue>,
) {
    let Some(sql) = text(v, "sql") else {
        return;
    };
    let refs = recover_references(sql, at, out);
    for reference in refs {
        let (cube, member) = match reference {
            Reference::Column(_) => continue,
            Reference::OwnMember(member) => (own, member),
            Reference::Member { cube, member } => {
                if ambiguous.contains(cube) {
                    continue;
                }
                let Some(index) = names.get(cube) else {
                    issue(out, at, "reference", &format!("Unknown cube `{cube}`"));
                    continue;
                };
                (&cubes[*index], member)
            }
        };
        if ["dimensions", "measures"]
            .iter()
            .any(|g| cube.get(*g).is_some_and(|v| !v.is_sequence()))
        {
            continue;
        }
        if !["dimensions", "measures"].iter().any(|g| {
            items(cube, g)
                .iter()
                .any(|m| text(m, "name") == Some(member))
        }) {
            issue(out, at, "reference", &format!("Unknown member `{member}`"));
        }
    }
}

// Recover only at brace boundaries; the shared parser still defines the grammar.
fn recover_references<'a>(
    mut sql: &'a str,
    at: &str,
    out: &mut Vec<ValidationIssue>,
) -> Vec<Reference<'a>> {
    let mut refs = Vec::new();
    loop {
        match locate_references(sql) {
            Ok(found) => {
                refs.extend(found.into_iter().map(|r| r.reference));
                break;
            }
            Err(ReferenceError::Malformed { at: offset }) => {
                if let Ok(prefix) = locate_references(&sql[..offset]) {
                    refs.extend(prefix.into_iter().map(|r| r.reference));
                }
                issue(out, at, "reference", "Malformed member reference");
                let rest = &sql[offset..];
                let Some(end) = rest.find('}') else {
                    break;
                };
                sql = &rest[end + 1..];
            }
            Err(_) => break,
        }
    }
    refs
}
