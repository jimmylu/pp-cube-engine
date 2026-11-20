//! Contract type for a query: Cube's query format, narrowed to what the
//! kernel compiles. A field it does not compile yet does not deserialize.

use std::fmt;

use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Members are written `cube.member`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Query {
    #[serde(default)]
    pub measures: Vec<String>,
    #[serde(default)]
    pub dimensions: Vec<String>,
    #[serde(default)]
    pub segments: Vec<String>,
    /// Top-level filters all hold together.
    #[serde(default)]
    pub filters: Vec<Filter>,
    /// In the sequence it was written, whichever of Cube's two forms was used.
    /// Absent means the planner's default order; written empty means no order.
    #[serde(
        default,
        deserialize_with = "read_order",
        serialize_with = "write_order",
        skip_serializing_if = "Option::is_none"
    )]
    pub order: Option<Vec<Order>>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

/// One condition, or a group of filters of which all or any must hold.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "FilterFields", into = "FilterFields")]
pub enum Filter {
    Condition(Condition),
    And(Vec<Filter>),
    Or(Vec<Filter>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Condition {
    pub member: String,
    pub operator: Operator,
    /// Strings, as Cube's format has them, numbers included.
    pub values: Vec<String>,
}

/// The operators that take plain values. The date operators are not compiled yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Operator {
    Equals,
    NotEquals,
    Contains,
    NotContains,
    StartsWith,
    NotStartsWith,
    EndsWith,
    NotEndsWith,
    Gt,
    Gte,
    Lt,
    Lte,
    Set,
    NotSet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Order {
    pub member: String,
    pub direction: Direction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Asc,
    Desc,
}

/// A filter as Cube writes it: one object that is a condition or a group.
#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FilterFields {
    #[serde(skip_serializing_if = "Option::is_none")]
    member: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    operator: Option<Operator>,
    #[serde(skip_serializing_if = "Option::is_none")]
    values: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    and: Option<Vec<Filter>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    or: Option<Vec<Filter>>,
}

impl TryFrom<FilterFields> for Filter {
    type Error = &'static str;

    fn try_from(fields: FilterFields) -> Result<Self, Self::Error> {
        match fields {
            FilterFields {
                member: Some(member),
                operator: Some(operator),
                values,
                and: None,
                or: None,
            } => Ok(Self::Condition(Condition {
                member,
                operator,
                values: values.unwrap_or_default(),
            })),
            FilterFields {
                member: None,
                operator: None,
                values: None,
                and: Some(filters),
                or: None,
            } => Ok(Self::And(filters)),
            FilterFields {
                member: None,
                operator: None,
                values: None,
                and: None,
                or: Some(filters),
            } => Ok(Self::Or(filters)),
            _ => Err(
                "a filter is either `member`, `operator` and `values`, or one of `and` and `or`",
            ),
        }
    }
}

impl From<Filter> for FilterFields {
    fn from(filter: Filter) -> Self {
        match filter {
            Filter::Condition(condition) => Self {
                member: Some(condition.member),
                operator: Some(condition.operator),
                values: Some(condition.values),
                ..Self::default()
            },
            Filter::And(filters) => Self {
                and: Some(filters),
                ..Self::default()
            },
            Filter::Or(filters) => Self {
                or: Some(filters),
                ..Self::default()
            },
        }
    }
}

/// Cube writes order as `{"member": "asc"}` or as `[["member", "asc"]]`.
/// Reading the object as a sequence of entries keeps the order it was written in.
fn read_order<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Vec<Order>>, D::Error> {
    struct Written;

    impl<'de> Visitor<'de> for Written {
        type Value = Vec<Order>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("an object of member and direction, or a list of such pairs")
        }

        fn visit_map<A: MapAccess<'de>>(self, mut entries: A) -> Result<Vec<Order>, A::Error> {
            let mut order = Vec::new();
            while let Some((member, direction)) = entries.next_entry()? {
                order.push(Order { member, direction });
            }
            Ok(order)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut pairs: A) -> Result<Vec<Order>, A::Error> {
            let mut order = Vec::new();
            while let Some((member, direction)) = pairs.next_element()? {
                order.push(Order { member, direction });
            }
            Ok(order)
        }
    }

    deserializer.deserialize_any(Written).map(Some)
}

#[expect(
    clippy::ref_option,
    reason = "serde passes the field by reference; absent is skipped before this runs"
)]
fn write_order<S: Serializer>(
    order: &Option<Vec<Order>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let order = order.as_deref().unwrap_or_default();
    let mut pairs = serializer.serialize_seq(Some(order.len()))?;
    for item in order {
        pairs.serialize_element(&(&item.member, item.direction))?;
    }
    pairs.end()
}
