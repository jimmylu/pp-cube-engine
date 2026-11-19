//! References inside an expression: `{CUBE}.column`, `{member}` and
//! `{cube.member}`. Any other use of braces is an error.

use std::fmt;

use crate::model::{Cube, Dimension, Measure, Model};

/// One reference, as written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reference<'a> {
    /// `{CUBE}.column`: a physical column of the cube the expression is in.
    Column(&'a str),
    /// `{member}`: a member of the cube the expression is in.
    OwnMember(&'a str),
    /// `{cube.member}`: a member of another cube.
    Member { cube: &'a str, member: &'a str },
}

/// What a reference points at in a model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Target<'a> {
    /// Taken as written: the model does not know the columns of a table.
    Column {
        cube: &'a Cube,
        column: &'a str,
    },
    Dimension(&'a Cube, &'a Dimension),
    Measure(&'a Cube, &'a Measure),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceError {
    /// The brace at byte `at` of the expression is not part of one of the three forms.
    Malformed {
        at: usize,
    },
    UnknownCube {
        cube: String,
    },
    UnknownMember {
        cube: String,
        member: String,
    },
}

impl fmt::Display for ReferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed { at } => write!(
                f,
                "the braces at offset {at} are not `{{CUBE}}.column`, `{{member}}` or `{{cube.member}}`"
            ),
            Self::UnknownCube { cube } => write!(f, "cube `{cube}` is not in the model"),
            Self::UnknownMember { cube, member } => {
                write!(f, "cube `{cube}` has no dimension or measure `{member}`")
            }
        }
    }
}

impl std::error::Error for ReferenceError {}

/// Finds the references in `expression`, in the order they are written.
pub fn parse_references(expression: &str) -> Result<Vec<Reference<'_>>, ReferenceError> {
    let mut found = Vec::new();
    let mut scanned = 0;
    while let Some(brace) = expression[scanned..].find(['{', '}']) {
        let at = scanned + brace;
        let malformed = ReferenceError::Malformed { at };
        // A closing brace found here has no opening brace before it.
        let closing = expression[at + 1..].find('}');
        let (true, Some(length)) = (expression[at..].starts_with('{'), closing) else {
            return Err(malformed);
        };
        let body = &expression[at + 1..at + 1 + length];
        scanned = at + length + 2;
        let reference = if body == "CUBE" {
            let column = expression[scanned..]
                .strip_prefix('.')
                .map(identifier_prefix)
                .filter(|column| !column.is_empty())
                .ok_or(malformed)?;
            scanned += 1 + column.len();
            Reference::Column(column)
        } else {
            match body.split_once('.') {
                None if is_identifier(body) => Reference::OwnMember(body),
                Some((cube, member))
                    if cube != "CUBE" && is_identifier(cube) && is_identifier(member) =>
                {
                    Reference::Member { cube, member }
                }
                _ => return Err(malformed),
            }
        };
        found.push(reference);
    }
    Ok(found)
}

fn is_identifier_char(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

fn is_identifier(text: &str) -> bool {
    !text.is_empty() && identifier_prefix(text).len() == text.len()
}

/// The longest identifier `text` starts with; empty when it starts with a digit.
fn identifier_prefix(text: &str) -> &str {
    if text.starts_with(|first: char| first.is_ascii_digit()) {
        return "";
    }
    let end = text.find(|character| !is_identifier_char(character));
    &text[..end.unwrap_or(text.len())]
}

impl Model {
    /// The cube called `name`.
    pub fn cube(&self, name: &str) -> Option<&Cube> {
        self.cubes.iter().find(|cube| cube.name == name)
    }

    /// Resolves every reference in `expression`, an expression of `cube`.
    pub fn resolve<'a>(
        &'a self,
        cube: &'a Cube,
        expression: &'a str,
    ) -> Result<Vec<Target<'a>>, ReferenceError> {
        parse_references(expression)?
            .into_iter()
            .map(|reference| match reference {
                Reference::Column(column) => Ok(Target::Column { cube, column }),
                Reference::OwnMember(member) => cube.member(member),
                Reference::Member { cube, member } => self
                    .cube(cube)
                    .ok_or_else(|| ReferenceError::UnknownCube {
                        cube: cube.to_owned(),
                    })?
                    .member(member),
            })
            .collect()
    }
}

impl Cube {
    fn member(&self, name: &str) -> Result<Target<'_>, ReferenceError> {
        let dimension = self
            .dimensions
            .iter()
            .find(|dimension| dimension.name == name);
        let measure = self.measures.iter().find(|measure| measure.name == name);
        match (dimension, measure) {
            (Some(dimension), _) => Ok(Target::Dimension(self, dimension)),
            (None, Some(measure)) => Ok(Target::Measure(self, measure)),
            (None, None) => Err(ReferenceError::UnknownMember {
                cube: self.name.clone(),
                member: name.to_owned(),
            }),
        }
    }
}
