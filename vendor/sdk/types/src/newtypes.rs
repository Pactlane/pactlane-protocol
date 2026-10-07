//! The Rust types a generated entity field takes.
//!
//! Each one wraps the encoding [`crate::Value`] uses, so generated structs are
//! self-describing and a mapping cannot accidentally put a `String` where a
//! `BigInt` belongs. They are intentionally thin: arithmetic on `BigInt` is
//! deferred until a mapping actually needs it (see the note on each type).

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

use serde::{Deserialize, Serialize};

use crate::scalar::Value;

/// An arbitrary-precision integer, carried as a decimal string.
///
/// Stored as a string rather than an `i128` because token amounts routinely
/// exceed 128 bits once decimals are involved. Arithmetic lands with the
/// bignum backend in a later milestone; for now the type exists so schemas,
/// generated code and the ABI agree on the encoding.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BigInt(String);

impl BigInt {
    /// Wrap a decimal string. No validation yet — see the type note.
    pub fn new(decimal: impl Into<String>) -> Self {
        Self(decimal.into())
    }

    /// The decimal representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BigInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<u64> for BigInt {
    fn from(v: u64) -> Self {
        Self(v.to_string())
    }
}

impl From<i64> for BigInt {
    fn from(v: i64) -> Self {
        Self(v.to_string())
    }
}

impl From<BigInt> for Value {
    fn from(v: BigInt) -> Self {
        Value::BigInt(v.0)
    }
}

/// An arbitrary-precision decimal, carried as a string.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BigDecimal(String);

impl BigDecimal {
    /// Wrap a decimal string.
    pub fn new(decimal: impl Into<String>) -> Self {
        Self(decimal.into())
    }

    /// The decimal representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BigDecimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<BigDecimal> for Value {
    fn from(v: BigDecimal) -> Self {
        Value::BigDecimal(v.0)
    }
}

/// Raw bytes, rendered as `0x`-prefixed hex everywhere user-visible.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Bytes(Vec<u8>);

impl Bytes {
    /// Wrap raw bytes.
    pub const fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// Parse `0x`-prefixed or bare hex.
    pub fn from_hex(s: &str) -> Result<Self, hex::FromHexError> {
        hex::decode(s.strip_prefix("0x").unwrap_or(s)).map(Self)
    }

    /// The `0x`-prefixed hex form.
    pub fn to_hex(&self) -> String {
        let mut out = String::from("0x");
        out.push_str(&hex::encode(&self.0));
        out
    }

    /// The underlying bytes.
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Display for Bytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl From<Bytes> for Value {
    fn from(v: Bytes) -> Self {
        Value::Bytes(v.0)
    }
}

/// A UTC instant, in milliseconds since the Unix epoch.
///
/// Milliseconds rather than seconds because sub-second ordering matters on
/// fast chains, and rather than nanoseconds because no chain in scope reports
/// them.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Timestamp(i64);

impl Timestamp {
    /// Wrap a millisecond epoch value.
    pub const fn from_millis(millis: i64) -> Self {
        Self(millis)
    }

    /// Build from a whole-second epoch value, as chains usually report.
    pub const fn from_secs(secs: i64) -> Self {
        Self(secs * 1_000)
    }

    /// Milliseconds since the epoch.
    pub const fn as_millis(self) -> i64 {
        self.0
    }
}

impl From<Timestamp> for Value {
    fn from(v: Timestamp) -> Self {
        Value::Date(v.0)
    }
}

/// An opaque JSON document stored in a `Json` field.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Json(serde_json::Value);

impl Json {
    /// Wrap a JSON value.
    pub const fn new(value: serde_json::Value) -> Self {
        Self(value)
    }

    /// The underlying value.
    pub const fn get(&self) -> &serde_json::Value {
        &self.0
    }
}

impl From<Json> for Value {
    fn from(v: Json) -> Self {
        Value::Json(v.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newtypes_lower_to_the_matching_value_variant() {
        assert_eq!(Value::from(BigInt::from(7u64)), Value::BigInt("7".into()));
        assert_eq!(
            Value::from(Bytes::from_hex("0xff").unwrap()),
            Value::Bytes(alloc::vec![0xff])
        );
        assert_eq!(Value::from(Timestamp::from_secs(2)), Value::Date(2_000));
    }

    #[test]
    fn bytes_round_trip_through_hex() {
        let b = Bytes::from_hex("0xdeadbeef").unwrap();
        assert_eq!(b.to_hex(), "0xdeadbeef");
        assert_eq!(Bytes::from_hex("deadbeef").unwrap(), b);
    }
}
