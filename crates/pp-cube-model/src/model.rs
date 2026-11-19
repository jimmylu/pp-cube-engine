//! Contract types: a Cube data model narrowed to the B0 baseline.
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
