//! A model kept as a directory, one cube or view per file, as Cube recommends:
//! `cubes/<name>.yml` and `views/<name>.yml`, each named after what it defines.
//!
//! Pure: the caller reads the files and hands their texts over.

use std::fmt;

use serde::Deserialize;

use crate::load::Position;
use crate::model::{Cube, Model, View};

/// One file of a model directory: its path inside the directory, with `/`
/// between its parts, and its text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelFile<'a> {
    pub path: &'a str,
    pub text: &'a str,
}

/// Why a directory is not a model. Each names the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssembleError {
    /// Not `cubes/<name>.yml` or `views/<name>.yml` with a snake_case name.
    UnexpectedFile { path: String },
    /// The file is not YAML of the contract.
    Syntax {
        path: String,
        message: String,
        position: Option<Position>,
    },
    /// A file under `cubes/` holds one cube and nothing else; under `views/`, one view.
    NotOne { path: String, kind: &'static str },
    /// The cube or view is not named after its file.
    Misnamed { path: String, found: String },
    /// A model has at least one cube.
    NoCubes,
}

impl fmt::Display for AssembleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedFile { path } => write!(
                f,
                "`{path}` is not `cubes/<name>.yml` or `views/<name>.yml` with a snake_case name"
            ),
            Self::Syntax { path, message, .. } => write!(f, "`{path}`: {message}"),
            Self::NotOne { path, kind } => {
                write!(
                    f,
                    "`{path}` must define exactly one {kind} and nothing else"
                )
            }
            Self::Misnamed { path, found } => {
                write!(
                    f,
                    "`{path}` defines `{found}`; a file is named after what it defines"
                )
            }
            Self::NoCubes => {
                f.write_str("a model directory needs at least one file under `cubes/`")
            }
        }
    }
}

impl std::error::Error for AssembleError {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Fragment {
    #[serde(default)]
    cubes: Vec<Cube>,
    #[serde(default)]
    views: Vec<View>,
}

fn snake_case(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes.next().is_some_and(|b| b.is_ascii_lowercase())
        && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// The folder and the name of `cubes/<name>.yml`, `views/<name>.yaml` and the like.
fn placed(path: &str) -> Option<(&str, &str)> {
    let (folder, file) = path.split_once('/')?;
    let name = file
        .strip_suffix(".yml")
        .or_else(|| file.strip_suffix(".yaml"))?;
    (matches!(folder, "cubes" | "views") && snake_case(name)).then_some((folder, name))
}

/// The model of a directory. Cubes come first and views after, each in the order
/// of their file names, so the same files always give the same model and digest.
pub fn assemble(files: &[ModelFile<'_>]) -> Result<Model, AssembleError> {
    let mut ordered: Vec<(&str, &str, &ModelFile<'_>)> = Vec::with_capacity(files.len());
    for file in files {
        let (folder, name) = placed(file.path).ok_or_else(|| AssembleError::UnexpectedFile {
            path: file.path.to_owned(),
        })?;
        ordered.push((folder, name, file));
    }
    ordered.sort_by_key(|(folder, name, _)| (*folder, *name));
    let mut model = Model {
        cubes: Vec::new(),
        views: Vec::new(),
    };
    for (folder, name, file) in ordered {
        let path = || file.path.to_owned();
        let fragment: Fragment =
            serde_norway::from_str(file.text).map_err(|error| AssembleError::Syntax {
                path: path(),
                position: error.location().map(|at| Position {
                    line: at.line(),
                    column: at.column(),
                }),
                message: error.to_string(),
            })?;
        let Fragment {
            mut cubes,
            mut views,
        } = fragment;
        let (found, kind) = match (folder, cubes.len(), views.len()) {
            ("cubes", 1, 0) => (cubes[0].name.clone(), "cube"),
            ("views", 0, 1) => (views[0].name.clone(), "view"),
            ("cubes", _, _) => {
                return Err(AssembleError::NotOne {
                    path: path(),
                    kind: "cube",
                })
            }
            _ => {
                return Err(AssembleError::NotOne {
                    path: path(),
                    kind: "view",
                })
            }
        };
        if found != name {
            return Err(AssembleError::Misnamed {
                path: path(),
                found,
            });
        }
        if kind == "cube" {
            model.cubes.append(&mut cubes);
        } else {
            model.views.append(&mut views);
        }
    }
    if model.cubes.is_empty() {
        return Err(AssembleError::NoCubes);
    }
    Ok(model)
}
