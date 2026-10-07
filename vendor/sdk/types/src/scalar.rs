//! The value model shared by the schema IR, the mapping ABI and the store.
//!
//! One representation, three consumers: the SDK encodes entity fields as
//! [`Value`], the node decodes them into column writes, and the query service
//! maps them back to GraphQL. Adding a variant here is a mapping ABI change.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

use serde::{Deserialize, Serialize};

/// The scalar types a schema field may declare.
///
/// These are the GraphQL scalar names a developer writes in `schema.graphql`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ScalarKind {
    /// `ID` — the entity primary key. Always a string at the storage layer.
    Id,
    /// `String`.
    String,
    /// `Boolean`.
    Boolean,
    /// `Int` — 32-bit signed, matching GraphQL's `Int`.
    Int,
    /// `BigInt` — arbitrary precision integer, carried as a decimal string.
    BigInt,
    /// `Float` — 64-bit IEEE-754.
    Float,
    /// `BigDecimal` — arbitrary precision decimal, carried as a string.
    BigDecimal,
    /// `Bytes` — raw bytes, rendered as `0x`-prefixed hex.
    Bytes,
    /// `Date` — a UTC instant, carried as milliseconds since the epoch.
    Date,
    /// `Json` — an opaque JSON document.
    Json,
}

impl ScalarKind {
    /// Every scalar the schema parser accepts.
    pub const ALL: &'static [ScalarKind] = &[
        ScalarKind::Id,
        ScalarKind::String,
        ScalarKind::Boolean,
        ScalarKind::Int,
        ScalarKind::BigInt,
        ScalarKind::Float,
        ScalarKind::BigDecimal,
        ScalarKind::Bytes,
        ScalarKind::Date,
        ScalarKind::Json,
    ];

    /// The GraphQL spelling of this scalar.
    pub const fn as_str(self) -> &'static str {
        match self {
            ScalarKind::Id => "ID",
            ScalarKind::String => "String",
            ScalarKind::Boolean => "Boolean",
            ScalarKind::Int => "Int",
            ScalarKind::BigInt => "BigInt",
            ScalarKind::Float => "Float",
            ScalarKind::BigDecimal => "BigDecimal",
            ScalarKind::Bytes => "Bytes",
            ScalarKind::Date => "Date",
            ScalarKind::Json => "Json",
        }
    }

    /// Resolve a GraphQL type name to a scalar, if it names one.
    ///
    /// Returns `None` for entity references and enums, which the schema
    /// resolver handles separately.
    pub fn from_graphql_name(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|s| s.as_str() == name)
    }
}

impl fmt::Display for ScalarKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A concrete entity field value crossing the mapping ABI.
///
/// The encoding is intentionally boring — arbitrary precision numbers travel
/// as strings so no host/guest pair has to agree on a bignum layout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
#[non_exhaustive]
pub enum Value {
    /// An absent value for a nullable field.
    Null,
    /// `String` and `ID`.
    String(String),
    /// `Boolean`.
    Boolean(bool),
    /// `Int`.
    Int(i32),
    /// `BigInt`, as a base-10 string.
    BigInt(String),
    /// `Float`.
    Float(f64),
    /// `BigDecimal`, as a decimal string.
    BigDecimal(String),
    /// `Bytes`, as `0x`-prefixed hex.
    Bytes(Vec<u8>),
    /// `Date`, as milliseconds since the Unix epoch.
    Date(i64),
    /// `Json`.
    Json(serde_json::Value),
    /// A list field of any of the above.
    List(Vec<Value>),
}

impl Value {
    /// Whether this value is [`Value::Null`].
    pub const fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// The scalar this value carries, or `None` for `Null` and `List`.
    pub const fn scalar_kind(&self) -> Option<ScalarKind> {
        Some(match self {
            Value::String(_) => ScalarKind::String,
            Value::Boolean(_) => ScalarKind::Boolean,
            Value::Int(_) => ScalarKind::Int,
            Value::BigInt(_) => ScalarKind::BigInt,
            Value::Float(_) => ScalarKind::Float,
            Value::BigDecimal(_) => ScalarKind::BigDecimal,
            Value::Bytes(_) => ScalarKind::Bytes,
            Value::Date(_) => ScalarKind::Date,
            Value::Json(_) => ScalarKind::Json,
            Value::Null | Value::List(_) => return None,
        })
    }

    /// Borrow the value as a string, for `String`/`ID` fields.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::String(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::String(v.to_string())
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Boolean(v)
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::Int(v)
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Float(v)
    }
}

impl From<Vec<u8>> for Value {
    fn from(v: Vec<u8>) -> Self {
        Value::Bytes(v)
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(v) => v.into(),
            None => Value::Null,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_names_round_trip() {
        for kind in ScalarKind::ALL {
            assert_eq!(ScalarKind::from_graphql_name(kind.as_str()), Some(*kind));
        }
        assert_eq!(ScalarKind::from_graphql_name("Transfer"), None);
    }

    #[test]
    fn values_are_tagged_so_the_host_never_guesses() {
        let json = serde_json::to_string(&Value::BigInt("1000".into())).unwrap();
        assert_eq!(json, r#"{"type":"bigInt","value":"1000"}"#);
        assert_eq!(
            serde_json::from_str::<Value>(&json).unwrap(),
            Value::BigInt("1000".into())
        );
    }

    #[test]
    fn none_becomes_null() {
        let v: Value = Option::<i32>::None.into();
        assert!(v.is_null());
        assert_eq!(Value::from(Some(7i32)), Value::Int(7));
    }
}
