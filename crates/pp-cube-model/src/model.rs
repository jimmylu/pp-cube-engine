//! Contract types: a Cube data model narrowed to the baseline.
//!
//! Plain data. A field or value outside the baseline does not deserialize.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

/// The cubes of one model document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Model {
    pub cubes: Vec<Cube>,
    /// Selections of members of joined cubes under one name. Not written when
    /// empty, so a model without views keeps its digest.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub views: Vec<View>,
}

/// A view: members of the cubes along its join paths, queried as its own.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct View {
    pub name: String,
    pub cubes: Vec<ViewCube>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public: Option<bool>,
    #[serde(
        default,
        deserialize_with = "crate::meta::deserialize",
        skip_serializing_if = "Option::is_none"
    )]
    pub meta: Option<Value>,
}

/// The members a view takes from the cube at the end of `join_path`, a chain of
/// joins written `root.joined.joined`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewCube {
    pub join_path: String,
    pub includes: Includes,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub excludes: Vec<String>,
    /// Whether the members are named `<cube>_<member>` in the view.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefix: Option<bool>,
}

/// `"*"` for every dimension and measure of the cube, or their names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Includes {
    All(AllMembers),
    Members(Vec<String>),
}

/// The text `*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AllMembers {
    #[serde(rename = "*")]
    All,
}

impl ViewCube {
    /// The cubes of the join path, root first.
    pub fn path(&self) -> Vec<&str> {
        self.join_path.split('.').collect()
    }

    /// The name in the view of the cube's member `member`.
    pub fn member_name(&self, member: &str) -> String {
        match (self.prefix, self.path().last()) {
            (Some(true), Some(cube)) => format!("{cube}_{member}"),
            _ => member.to_owned(),
        }
    }

    fn takes(&self, member: &str) -> bool {
        let included = match &self.includes {
            Includes::All(_) => true,
            Includes::Members(names) => names.iter().any(|name| name == member),
        };
        included && !self.excludes.iter().any(|name| name == member)
    }
}

impl Model {
    /// Whether `name` is a view of this model.
    pub fn is_view(&self, name: &str) -> bool {
        self.views.iter().any(|view| view.name == name)
    }

    /// The model with each view also present as a cube whose members refer to
    /// the members it takes, by join path: the form the planner works on. A
    /// view cube has no table; its measures are formulas over the measures it
    /// takes. A view that names an unknown cube takes nothing from it, which
    /// validation reports.
    pub fn expanded(&self) -> Self {
        let mut expanded = self.clone();
        for view in &self.views {
            if self.cube(&view.name).is_some() {
                continue;
            }
            let mut cube = Cube {
                name: view.name.clone(),
                sql_table: None,
                sql: None,
                data_source: String::new(),
                sql_alias: None,
                title: view.title.clone(),
                description: view.description.clone(),
                public: view.public,
                meta: view.meta.clone(),
                joins: Vec::new(),
                dimensions: Vec::new(),
                measures: Vec::new(),
                segments: Vec::new(),
            };
            for taken in &view.cubes {
                let path = taken.path();
                let Some(source) = path.last().and_then(|name| self.cube(name)) else {
                    continue;
                };
                if cube.data_source.is_empty() {
                    cube.data_source.clone_from(&source.data_source);
                }
                let reference = |member: &str| format!("{{{}.{member}}}", taken.join_path);
                for dimension in source.dimensions.iter().filter(|d| taken.takes(&d.name)) {
                    cube.dimensions.push(Dimension {
                        name: taken.member_name(&dimension.name),
                        sql: reference(&dimension.name),
                        primary_key: false,
                        ..dimension.clone()
                    });
                }
                for measure in source.measures.iter().filter(|m| taken.takes(&m.name)) {
                    cube.measures.push(Measure {
                        name: taken.member_name(&measure.name),
                        measure_type: MeasureType::Number,
                        sql: Some(reference(&measure.name)),
                        filters: Vec::new(),
                        rolling_window: None,
                        time_shift: Vec::new(),
                        ..measure.clone()
                    });
                }
            }
            expanded.cubes.push(cube);
        }
        expanded
    }
}

/// One cube. `sql_table` and `sql` are kept as written; that exactly one of
/// them is set is a validation rule, not a loading rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cube {
    pub name: String,
    pub sql_table: Option<String>,
    pub sql: Option<String>,
    pub data_source: String,
    pub sql_alias: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub public: Option<bool>,
    /// Kept as written; the kernel does not interpret it.
    #[serde(default, deserialize_with = "crate::meta::deserialize")]
    pub meta: Option<Value>,
    #[serde(default)]
    pub joins: Vec<Join>,
    #[serde(default)]
    pub dimensions: Vec<Dimension>,
    #[serde(default)]
    pub measures: Vec<Measure>,
    #[serde(default)]
    pub segments: Vec<Segment>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dimension {
    pub name: String,
    pub sql: String,
    #[serde(rename = "type")]
    pub dimension_type: DimensionType,
    #[serde(default)]
    pub primary_key: bool,
    pub title: Option<String>,
    pub description: Option<String>,
    pub public: Option<bool>,
    #[serde(default, deserialize_with = "crate::meta::deserialize")]
    pub meta: Option<Value>,
    /// Granularities of a time dimension besides the predefined ones. Not
    /// written when empty, so a model without them keeps its digest.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub granularities: Vec<CustomGranularity>,
}

/// Periods of `interval`, `<n> <unit>`, counted from `origin`, or natural
/// periods moved by `offset`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustomGranularity {
    pub name: String,
    pub interval: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DimensionType {
    String,
    Number,
    Boolean,
    Time,
}

// `remote = "Self"` makes the derives emit inherent `serialize` and
// `deserialize` functions instead of the trait impls, so the impls below can
// wrap them and check the one rule that spans two fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self", deny_unknown_fields)]
pub struct Measure {
    pub name: String,
    #[serde(rename = "type")]
    pub measure_type: MeasureType,
    /// The unaggregated expression, or for `number` a formula over other
    /// measures. Only `count` may omit it.
    pub sql: Option<String>,
    #[serde(default)]
    pub filters: Vec<MeasureFilter>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub public: Option<bool>,
    #[serde(default, deserialize_with = "crate::meta::deserialize")]
    pub meta: Option<Value>,
    pub format: Option<String>,
    /// Aggregates over a window of time ending at each row's period.
    /// Absent fields are not written, so a model without them keeps its digest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rolling_window: Option<RollingWindow>,
    /// Computed from other measures after they are aggregated. Needed by `time_shift`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multi_stage: Option<bool>,
    /// Reads the measures it is computed from at another time.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub time_shift: Vec<TimeShift>,
}

/// The window of a rolling measure: `trailing` and `leading` as `<n> <unit>` or
/// `unbounded`, or `type: to_date` with the `granularity` the period starts at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RollingWindow {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trailing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leading: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<String>,
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub window_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub granularity: Option<String>,
}

/// One shift of a multi-stage measure: `interval` as `<n> <unit>`, `type` prior
/// or next, along `time_dimension` of the measure's cube or, when absent, along
/// the time dimension of the query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeShift {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_dimension: Option<String>,
    pub interval: String,
    #[serde(rename = "type")]
    pub shift_type: String,
}

impl<'de> Deserialize<'de> for Measure {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let measure = Self::deserialize(deserializer)?;
        if measure.sql.is_none() && measure.measure_type != MeasureType::Count {
            return Err(D::Error::custom(format_args!(
                "measure `{}` needs `sql`; only a count measure may omit it",
                measure.name
            )));
        }
        Ok(measure)
    }
}

impl Serialize for Measure {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        Self::serialize(self, serializer)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasureType {
    Count,
    CountDistinct,
    /// An estimate of the distinct count, by the engine's own sketch.
    CountDistinctApprox,
    Sum,
    Avg,
    Min,
    Max,
    Number,
}

/// One condition of a filtered measure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeasureFilter {
    pub sql: String,
}

/// A join to the cube called `name`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Join {
    pub name: String,
    pub relationship: Relationship,
    pub sql: String,
}

/// The baseline allows only these two relationships.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Relationship {
    ManyToOne,
    OneToOne,
}

/// A predefined filter condition, referenced by name in a query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Segment {
    pub name: String,
    pub sql: String,
}
