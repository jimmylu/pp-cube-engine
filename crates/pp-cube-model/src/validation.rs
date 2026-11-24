//! Accumulated structural and baseline SQL diagnostics.
use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;
use serde_norway::Value;

use crate::{locate_references, Model, Reference, ReferenceError};

/// A model validation fault. Paths are JSON Pointers; the empty path is the root.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ValidationIssue {
    pub path: String,
    /// Stable machine-readable rule, independent of the human message.
    pub rule_id: String,
    pub message: String,
}

pub(super) fn issue(out: &mut Vec<ValidationIssue>, path: &str, rule: &str, message: &str) {
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
/// Validates YAML structure and baseline SQL constraints.
pub fn validate_yaml(text: &str) -> Vec<ValidationIssue> {
    match serde_norway::from_str(text) {
        Ok(value) => validate(value),
        Err(_) => syntax(),
    }
}
/// Validates strict JSON and baseline SQL, retaining the loader's parser settings.
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
/// cannot survive deserialization. Baseline SQL restrictions are checked.
pub fn validate_model(model: &Model) -> Vec<ValidationIssue> {
    match serde_norway::to_value(model) {
        Ok(value) => validate(value),
        Err(_) => syntax(),
    }
}

pub(super) fn text<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key)?.as_str()
}
pub(super) fn items<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v.get(key)
        .and_then(Value::as_sequence)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn validate(value: Value) -> Vec<ValidationIssue> {
    let mut out = Vec::new();
    shape(&value, "", "model", &mut out);
    semantics(&value, &mut out);
    crate::sql_validation::validate(&value, &mut out);
    out.sort();
    out.dedup();
    out
}

// Required fields, optional fields, nested collection element kinds.
fn fields(kind: &str) -> (&str, &str, &[(&str, &str)]) {
    match kind {
        "model" => ("cubes", "views", &[("cubes", "cube"), ("views", "view")]),
        "view" => ("name cubes", "title description public meta", &[("cubes", "view_cube")]),
        "view_cube" => ("join_path includes", "excludes prefix", &[]),
        "cube" => ("name data_source", "sql_table sql sql_alias title description public meta joins dimensions measures segments", &[("joins","join"),("dimensions","dimension"),("measures","measure"),("segments","segment")]),
        "dimension" => ("name sql type", "primary_key title description public meta granularities", &[("granularities","granularity")]),
        "granularity" => ("name interval", "origin offset", &[]),
        "measure" => ("name type", "sql filters title description public meta format rolling_window multi_stage time_shift", &[("filters","filter"),("time_shift","time_shift")]),
        "rolling_window" => ("", "trailing leading offset type granularity", &[]),
        "time_shift" => ("interval type", "time_dimension", &[]),
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
        } else if kind == "view_cube" && (key == "includes" || key == "excludes") {
            let names = val.as_sequence().is_some_and(|names| {
                names
                    .iter()
                    .all(|name| name.as_str().is_some_and(|n| !n.is_empty()))
            });
            if !(names || key == "includes" && val.as_str() == Some("*")) {
                issue(
                    out,
                    &here,
                    "type",
                    "Expected a list of member names, or `*` to include all",
                );
            }
        } else if key == "rolling_window" && !val.is_null() {
            shape(val, &here, "rolling_window", out);
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
                "public" | "primary_key" | "multi_stage" | "prefix" => val.as_bool().is_some(),
                "join_path" => val.as_str().is_some_and(|path| {
                    path.split('.')
                        .all(|cube| crate::Granularity::is_custom_name(&cube.to_ascii_lowercase()))
                }),
                "type" if kind == "rolling_window" => val.as_str() == Some("to_date"),
                "type" if kind == "time_shift" => {
                    val.as_str().is_some_and(|s| ["prior", "next"].contains(&s))
                }
                "offset" if kind == "granularity" => val.as_str().is_some_and(interval),
                "offset" => val.as_str().is_some_and(|s| ["start", "end"].contains(&s)),
                "origin" => val
                    .as_str()
                    .is_some_and(|s| crate::query::DateValue::try_from(s.to_owned()).is_ok()),
                "name" if kind == "granularity" => val.as_str().is_some_and(|s| {
                    crate::Granularity::is_custom_name(s)
                        && ![
                            "second", "minute", "hour", "day", "week", "month", "quarter", "year",
                        ]
                        .contains(&s)
                }),
                "granularity" => val
                    .as_str()
                    .is_some_and(|s| ["day", "week", "month", "quarter", "year"].contains(&s)),
                "interval" => val.as_str().is_some_and(interval),
                "trailing" | "leading" => val
                    .as_str()
                    .is_some_and(|s| s == "unbounded" || interval(s)),
                "type" if kind == "dimension" => val
                    .as_str()
                    .is_some_and(|s| ["string", "number", "boolean", "time"].contains(&s)),
                "type" => val.as_str().is_some_and(|s| {
                    [
                        "count",
                        "count_distinct",
                        "count_distinct_approx",
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
/// `<n> <unit>`: a positive whole number of days, weeks, months, quarters or years.
fn interval(text: &str) -> bool {
    let mut parts = text.split(' ');
    let (Some(count), Some(unit), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    count.len() <= 4
        && !count.starts_with('0')
        && count.bytes().all(|b| b.is_ascii_digit())
        && !count.is_empty()
        && ["day", "week", "month", "quarter", "year"].contains(&unit)
}

/// The rules that span the fields of a rolling or shifted measure.
fn windows(cube: &Value, measure: &Value, at: &str, out: &mut Vec<ValidationIssue>) {
    let set = |key: &str| measure.get(key).is_some_and(|v| !v.is_null());
    let kind = text(measure, "type");
    let multi_stage = measure.get("multi_stage").and_then(Value::as_bool) == Some(true);
    if let Some(window) = measure.get("rolling_window").filter(|v| v.is_mapping()) {
        let has = |key: &str| window.get(key).is_some_and(|v| !v.is_null());
        let here = path(at, "rolling_window");
        if matches!(kind, Some("number" | "count_distinct_approx")) {
            issue(
                out,
                &here,
                "window",
                "This measure type cannot have a rolling window",
            );
        }
        let to_date = has("type");
        let bounded = has("trailing") || has("leading") || has("offset");
        if to_date && (bounded || !has("granularity")) {
            issue(
                out,
                &here,
                "window",
                "A to_date window has a granularity and no bounds",
            );
        }
        if !to_date && (has("granularity") || !(has("trailing") || has("leading"))) {
            issue(
                out,
                &here,
                "window",
                "A window needs trailing or leading, or type to_date",
            );
        }
        if multi_stage || set("time_shift") {
            issue(
                out,
                &here,
                "window",
                "A rolling window cannot be multi-stage or shifted",
            );
        }
    }
    if multi_stage && kind.is_some_and(|kind| kind != "number") {
        issue(
            out,
            &path(at, "multi_stage"),
            "multi-stage",
            "Only a number measure can be multi-stage",
        );
    }
    let shifts = items(measure, "time_shift");
    if !shifts.is_empty() && !multi_stage {
        issue(
            out,
            &path(at, "time_shift"),
            "multi-stage",
            "A time shift needs multi_stage: true",
        );
    }
    if shifts.len() > 1 {
        issue(
            out,
            &path(at, "time_shift"),
            "time-shift",
            "One time shift per measure is supported",
        );
    }
    for (k, shift) in shifts.iter().enumerate() {
        let Some(name) = text(shift, "time_dimension") else {
            continue;
        };
        let known = items(cube, "dimensions")
            .iter()
            .any(|d| text(d, "name") == Some(name) && text(d, "type") == Some("time"));
        if !known {
            issue(
                out,
                &format!("{at}/time_shift/{k}/time_dimension"),
                "time-shift",
                "A time shift names a time dimension of its own cube",
            );
        }
    }
}

/// The rules that span the fields of a dimension's granularities.
fn granularities(dimension: &Value, at: &str, out: &mut Vec<ValidationIssue>) {
    let declared = items(dimension, "granularities");
    if !declared.is_empty() && text(dimension, "type").is_some_and(|kind| kind != "time") {
        let here = path(at, "granularities");
        issue(
            out,
            &here,
            "granularity",
            "Only a time dimension has granularities",
        );
    }
    let mut names = BTreeSet::new();
    for (k, granularity) in declared.iter().enumerate() {
        let here = format!("{at}/granularities/{k}");
        if text(granularity, "name").is_some_and(|name| !names.insert(name)) {
            issue(
                out,
                &path(&here, "name"),
                "duplicate-name",
                "Granularity name is repeated",
            );
        }
        let has = |key: &str| granularity.get(key).is_some_and(|v| !v.is_null());
        // Periods of more than one unit are counted from somewhere; without an
        // origin that would be the start of the year the query is compiled in.
        let one_unit = text(granularity, "interval").is_none_or(|i| i.starts_with("1 "));
        if has("origin") && has("offset") {
            issue(
                out,
                &here,
                "granularity",
                "A granularity has an origin or an offset, not both",
            );
        } else if !one_unit && !has("origin") {
            issue(
                out,
                &here,
                "granularity",
                "An interval of several units needs an origin",
            );
        }
    }
}

/// The rules of views: each takes existing members of cubes reached by declared
/// joins from one root, under names that are unique in the view.
fn views(value: &Value, out: &mut Vec<ValidationIssue>) {
    let cubes = items(value, "cubes");
    let cube = |name: &str| cubes.iter().find(|cube| text(cube, "name") == Some(name));
    let mut names = BTreeSet::new();
    for (i, view) in items(value, "views").iter().enumerate() {
        let base = format!("/views/{i}");
        if let Some(name) = text(view, "name") {
            if cube(name).is_some() || !names.insert(name) {
                issue(
                    out,
                    &path(&base, "name"),
                    "duplicate-name",
                    "View name is taken",
                );
            }
        }
        let taken = items(view, "cubes");
        if taken.is_empty() && view.get("cubes").is_some_and(Value::is_sequence) {
            issue(
                out,
                &path(&base, "cubes"),
                "view",
                "A view takes members of a cube",
            );
        }
        let mut root = None;
        let mut members = BTreeSet::new();
        for (j, entry) in taken.iter().enumerate() {
            let at = format!("{base}/cubes/{j}");
            let Some(join_path) = text(entry, "join_path") else {
                continue;
            };
            let mut steps = join_path.split('.');
            let first = steps.next().unwrap_or_default();
            if *root.get_or_insert(first) != first {
                issue(
                    out,
                    &path(&at, "join_path"),
                    "view",
                    "The join paths of a view start at one cube",
                );
            }
            // Each step is a join the cube before it declares.
            let mut current = cube(first);
            for step in steps {
                current = current
                    .filter(|from| {
                        items(from, "joins")
                            .iter()
                            .any(|j| text(j, "name") == Some(step))
                    })
                    .and_then(|_| cube(step));
            }
            let Some(source) = current else {
                issue(
                    out,
                    &path(&at, "join_path"),
                    "view",
                    "The join path is not a chain of declared joins",
                );
                continue;
            };
            let declared = |group: &str, name: &str| {
                items(source, group)
                    .iter()
                    .any(|m| text(m, "name") == Some(name))
            };
            let known = |name: &str| declared("dimensions", name) || declared("measures", name);
            for key in ["includes", "excludes"] {
                for (k, name) in items(entry, key).iter().enumerate() {
                    if name.as_str().is_some_and(|name| !known(name)) {
                        issue(
                            out,
                            &format!("{at}/{key}/{k}"),
                            "view",
                            "A view takes dimensions and measures of the cube its join path ends at",
                        );
                    }
                }
            }
            let excluded = |name: &str| {
                items(entry, "excludes")
                    .iter()
                    .any(|e| e.as_str() == Some(name))
            };
            let all = text(entry, "includes") == Some("*");
            let prefixed = entry.get("prefix").and_then(Value::as_bool) == Some(true);
            for group in ["dimensions", "measures"] {
                for member in items(source, group) {
                    let Some(name) = text(member, "name") else {
                        continue;
                    };
                    let included = all
                        || items(entry, "includes")
                            .iter()
                            .any(|n| n.as_str() == Some(name));
                    if !included || excluded(name) {
                        continue;
                    }
                    let last = join_path.rsplit('.').next().unwrap_or(join_path);
                    let exposed = if prefixed {
                        format!("{last}_{name}")
                    } else {
                        name.to_owned()
                    };
                    if !members.insert(exposed) {
                        issue(
                            out,
                            &at,
                            "duplicate-name",
                            "Two members of the view have one name; use `prefix` or `excludes`",
                        );
                    }
                }
            }
        }
    }
}

fn semantics(value: &Value, out: &mut Vec<ValidationIssue>) {
    views(value, out);
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
                if group == "measures" {
                    windows(cube, member, &at, out);
                }
                if group == "dimensions" {
                    granularities(member, &at, out);
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
            Reference::Joined { .. } => {
                issue(
                    out,
                    at,
                    "reference",
                    "A reference is `{CUBE}.column`, `{member}` or `{cube.member}`",
                );
                continue;
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
