//! Dialect templates and functions for Presto, Doris and Druid.
//!
//! A dialect is data: named templates taken from Cube at the pinned commit,
//! plus a few settings. Nothing here knows about the planner.

use std::collections::HashMap;
use std::fmt;
use std::sync::LazyLock;

use minijinja::{Environment, Value};
use serde::Deserialize;

/// The engines the kernel can compile for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DialectId {
    Presto,
    Doris,
}

impl DialectId {
    /// The name callers use for this dialect in requests and configuration.
    pub fn name(self) -> &'static str {
        match self {
            Self::Presto => "presto",
            Self::Doris => "doris",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        [Self::Presto, Self::Doris]
            .into_iter()
            .find(|id| id.name() == name)
    }
}

/// An engine session setting the compiled SQL needs. The kernel cannot set it;
/// whoever executes the SQL must.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SessionProperty {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialectError {
    /// The dialect has no template of that name yet.
    UnknownTemplate {
        dialect: &'static str,
        template: String,
    },
    /// A template did not parse or did not render.
    Template { template: String, message: String },
    /// The dialect's data file is not what this crate expects.
    Data {
        dialect: &'static str,
        message: String,
    },
}

impl fmt::Display for DialectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownTemplate { dialect, template } => {
                write!(f, "dialect {dialect} has no template `{template}`")
            }
            Self::Template { template, message } => write!(f, "template `{template}`: {message}"),
            Self::Data { dialect, message } => write!(f, "dialect {dialect} data: {message}"),
        }
    }
}

impl std::error::Error for DialectError {}

/// One engine's templates and settings.
#[derive(Debug)]
pub struct Dialect {
    name: &'static str,
    sources: HashMap<String, String>,
    engine: Environment<'static>,
    reuses_parameters: bool,
    offset_session: &'static [(&'static str, &'static str)],
    timestamp_precision: u32,
    interval_units: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DataFile {
    /// The upstream commit the templates were taken from; for readers of the file.
    #[serde(rename = "commit")]
    _commit: String,
    /// The digits after the second in a timestamp the engine is given.
    timestamp_precision: u32,
    /// The units an interval literal of the engine can carry.
    interval_units: Vec<String>,
    templates: HashMap<String, TemplateEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TemplateEntry {
    #[serde(rename = "source")]
    _source: String,
    text: String,
}

/// Presto renders `?` for every parameter, so one value cannot be bound twice.
static PRESTO: LazyLock<Result<Dialect, DialectError>> = LazyLock::new(|| {
    Dialect::from_data(
        "presto",
        include_str!("../../../templates/presto.yaml"),
        false,
        // PrestoDB answers "Offset support is not enabled" to the OFFSET clause otherwise.
        &[("offset_clause_enabled", "true")],
    )
});

static DORIS: LazyLock<Result<Dialect, DialectError>> = LazyLock::new(|| {
    Dialect::from_data(
        "doris",
        include_str!("../../../templates/doris.yaml"),
        false,
        &[],
    )
});

/// The dialect for `id`. Its data is parsed once per process.
pub fn dialect(id: DialectId) -> Result<&'static Dialect, DialectError> {
    match id {
        DialectId::Presto => PRESTO.as_ref().map_err(Clone::clone),
        DialectId::Doris => DORIS.as_ref().map_err(Clone::clone),
    }
}

impl Dialect {
    fn from_data(
        name: &'static str,
        data: &str,
        reuses_parameters: bool,
        offset_session: &'static [(&'static str, &'static str)],
    ) -> Result<Self, DialectError> {
        let file: DataFile = serde_norway::from_str(data).map_err(|error| DialectError::Data {
            dialect: name,
            message: error.to_string(),
        })?;
        let mut engine = Environment::new();
        let mut sources = HashMap::with_capacity(file.templates.len());
        for (template, entry) in file.templates {
            engine
                .add_template_owned(template.clone(), entry.text.clone())
                .map_err(|error| DialectError::Template {
                    template: template.clone(),
                    message: error.to_string(),
                })?;
            sources.insert(template, entry.text);
        }
        Ok(Self {
            name,
            sources,
            engine,
            reuses_parameters,
            offset_session,
            timestamp_precision: file.timestamp_precision,
            interval_units: file.interval_units,
        })
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The text of a template, for the ones used as they are, such as quotes.
    pub fn template(&self, name: &str) -> Option<&String> {
        self.sources.get(name)
    }

    /// Renders the template `name` with `context`.
    pub fn render(&self, name: &str, context: Value) -> Result<String, DialectError> {
        if !self.sources.contains_key(name) {
            return Err(DialectError::UnknownTemplate {
                dialect: self.name,
                template: name.to_owned(),
            });
        }
        let failed = |error: minijinja::Error| DialectError::Template {
            template: name.to_owned(),
            message: error.to_string(),
        };
        self.engine
            .get_template(name)
            .map_err(failed)?
            .render(context)
            .map_err(failed)
    }

    /// Whether one bound value may be referred to by several placeholders.
    pub fn reuses_parameters(&self) -> bool {
        self.reuses_parameters
    }

    /// The session properties SQL for a query needs on this engine; `offset`
    /// says whether the query sets an offset. Empty when it needs none.
    pub fn session_properties(&self, offset: bool) -> Vec<SessionProperty> {
        let needed = if offset { self.offset_session } else { &[] };
        needed
            .iter()
            .map(|(name, value)| SessionProperty {
                name: (*name).to_owned(),
                value: (*value).to_owned(),
            })
            .collect()
    }

    pub fn timestamp_precision(&self) -> u32 {
        self.timestamp_precision
    }

    /// Whether the engine has an interval literal of `unit`, such as `week`.
    pub fn has_interval_unit(&self, unit: &str) -> bool {
        self.interval_units.iter().any(|known| known == unit)
    }

    /// Whether the time series of a rolling window is generated by the engine
    /// from the bounds of the range, rather than written out period by period.
    pub fn generates_time_series(&self) -> bool {
        self.sources
            .contains_key("statements/generated_time_series_select")
    }
}
