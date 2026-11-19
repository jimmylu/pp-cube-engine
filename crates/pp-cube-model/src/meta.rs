//! Deserializing `meta`: any JSON-shaped value, kept as written.
//!
//! `serde_json::Value`'s own deserializer turns a non-finite float into null
//! and takes whatever float a parser made of an integer too long for 64 bits.
//! Here a number that cannot be kept exactly is an error in every format.

use std::fmt;

use serde::de::{DeserializeSeed, Deserializer, Error, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};

/// A float at or beyond 2^63 is what an integer outside the 64-bit range
/// becomes in a JSON parser, already rounded.
const FLOAT_LIMIT: f64 = 9_223_372_036_854_775_808.0;

/// For `#[serde(default, deserialize_with = ...)]` on a `meta` field.
pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Value>, D::Error> {
    let value = deserializer.deserialize_any(MetaValue)?;
    Ok(Some(value).filter(|value| !value.is_null()))
}

struct MetaValue;

fn inexact<E: Error>(number: impl fmt::Display) -> E {
    E::custom(format_args!(
        "metadata number {number} cannot be kept exactly; write it as a string"
    ))
}

impl<'de> DeserializeSeed<'de> for MetaValue {
    type Value = Value;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for MetaValue {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a metadata value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
        Ok(Value::from(value))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Value, E> {
        Ok(Value::from(value))
    }

    fn visit_i128<E: Error>(self, value: i128) -> Result<Value, E> {
        i64::try_from(value)
            .map(Value::from)
            .map_err(|_| inexact(value))
    }

    fn visit_u128<E: Error>(self, value: u128) -> Result<Value, E> {
        u64::try_from(value)
            .map(Value::from)
            .map_err(|_| inexact(value))
    }

    fn visit_f64<E: Error>(self, value: f64) -> Result<Value, E> {
        Number::from_f64(value)
            .filter(|_| value.abs() < FLOAT_LIMIT)
            .map(Value::Number)
            .ok_or_else(|| inexact(value))
    }

    fn visit_str<E>(self, value: &str) -> Result<Value, E> {
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Value, E> {
        Ok(Value::String(value))
    }

    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut items: A) -> Result<Value, A::Error> {
        let mut array = Vec::new();
        while let Some(item) = items.next_element_seed(MetaValue)? {
            array.push(item);
        }
        Ok(Value::Array(array))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut entries: A) -> Result<Value, A::Error> {
        let mut object = Map::new();
        while let Some(key) = entries.next_key::<String>()? {
            object.insert(key, entries.next_value_seed(MetaValue)?);
        }
        Ok(Value::Object(object))
    }
}
