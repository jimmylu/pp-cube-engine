//! Contract type for a query: Cube's query format, narrowed to what the
//! kernel compiles. A field it does not compile yet does not deserialize.

use std::fmt;

use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Members are written `cube.member`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
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
    #[serde(default)]
    pub time_dimensions: Vec<TimeDimension>,
    /// An IANA name. Absent means the timezone of the compile context.
    pub timezone: Option<String>,
}

/// A time dimension to group by, to restrict to a range, or both.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TimeDimension {
    pub dimension: String,
    pub granularity: Option<Granularity>,
    pub date_range: Option<DateRange>,
}

/// The granularities of the baseline. Cube also has the ones below a day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Granularity {
    Day,
    Week,
    Month,
    Quarter,
    Year,
}

/// A closed range: both ends belong to it. Written as a list of two.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(into = "(DateValue, DateValue)")]
pub struct DateRange {
    pub from: DateValue,
    pub to: DateValue,
}

impl From<DateRange> for (DateValue, DateValue) {
    fn from(range: DateRange) -> Self {
        (range.from, range.to)
    }
}

impl<'de> Deserialize<'de> for DateRange {
    /// Cube also takes one string here: a single day, or a relative range
    /// such as "last month" that it resolves from the clock. The kernel
    /// reads no clock, so the caller resolves those.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Written;

        impl<'de> Visitor<'de> for Written {
            type Value = DateRange;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a list of two dates, the start and the end of the range")
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut dates: A) -> Result<DateRange, A::Error> {
                let mut next = |seen| {
                    dates
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(seen, &self))
                };
                let range = DateRange {
                    from: next(0)?,
                    to: next(1)?,
                };
                match dates.next_element::<de::IgnoredAny>()? {
                    None => Ok(range),
                    Some(_) => Err(de::Error::invalid_length(3, &self)),
                }
            }
        }

        deserializer.deserialize_any(Written)
    }
}

/// A date `YYYY-MM-DD`, or a date and a time `YYYY-MM-DDTHH:mm:ss` with
/// optional `.SSS`. It has no zone: it is read in the query's timezone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DateValue(String);

impl DateValue {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether `text` is one of the forms above and names a real day and time.
    fn is_valid(text: &str) -> bool {
        let bytes = text.as_bytes();
        // The digits of `bytes[at..at + width]`, or `None` if any is not one.
        let number = |at: usize, width: usize| {
            bytes
                .get(at..at + width)?
                .iter()
                .try_fold(0_u32, |sum, byte| {
                    byte.is_ascii_digit()
                        .then(|| sum * 10 + u32::from(byte - b'0'))
                })
        };
        let separators = |expected: &[(usize, u8)]| {
            expected
                .iter()
                .all(|(at, separator)| bytes.get(*at) == Some(separator))
        };
        let date = || {
            let (year, month, day) = (number(0, 4)?, number(5, 2)?, number(8, 2)?);
            let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
            let days = match month {
                1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
                4 | 6 | 9 | 11 => 30,
                2 if leap => 29,
                2 => 28,
                _ => return None,
            };
            Some(separators(&[(4, b'-'), (7, b'-')]) && (1..=days).contains(&day))
        };
        let time = || {
            let (hour, minute, second) = (number(11, 2)?, number(14, 2)?, number(17, 2)?);
            Some(
                separators(&[(10, b'T'), (13, b':'), (16, b':')])
                    && hour < 24
                    && minute < 60
                    && second < 60,
            )
        };
        let fraction = || Some(separators(&[(19, b'.')]) && number(20, 3).is_some());
        let valid = match bytes.len() {
            10 => date(),
            19 => date().zip(time()).map(|(date, time)| date && time),
            23 => date()
                .zip(time())
                .zip(fraction())
                .map(|((date, time), fraction)| date && time && fraction),
            _ => None,
        };
        valid.unwrap_or(false)
    }
}

impl TryFrom<String> for DateValue {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        if Self::is_valid(&text) {
            Ok(Self(text))
        } else {
            Err(format!(
                "`{text}` is not a date: write `YYYY-MM-DD` or `YYYY-MM-DDTHH:mm:ss.SSS`, \
                 without a zone; relative dates are resolved by the caller"
            ))
        }
    }
}

impl From<DateValue> for String {
    fn from(date: DateValue) -> Self {
        date.0
    }
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
    /// Strings, as Cube's format has them, numbers and dates included.
    pub values: Vec<String>,
}

/// The operators of the baseline: those on plain values, then those on dates.
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
    InDateRange,
    NotInDateRange,
    BeforeDate,
    BeforeOrOnDate,
    AfterDate,
    AfterOrOnDate,
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
