//! Manifest specification version.
//!
//! The spec version is the contract between a project on disk and the tools
//! that read it. It is deliberately a small, explicit enum rather than a free
//! semver string: an unknown version must be a hard, legible error, never a
//! best-effort parse.

use alloc::string::String;
use core::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A supported `specVersion` value from `project.yaml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[non_exhaustive]
pub enum SpecVersion {
    /// `specVersion: "1.0"` — the initial public manifest format.
    #[default]
    V1_0,
}

impl SpecVersion {
    /// Every version this build understands, oldest first.
    pub const ALL: &'static [SpecVersion] = &[SpecVersion::V1_0];

    /// The newest version this build understands. `superquery init` writes it.
    pub const LATEST: SpecVersion = SpecVersion::V1_0;

    /// The wire representation written to and read from manifests.
    pub const fn as_str(self) -> &'static str {
        match self {
            SpecVersion::V1_0 => "1.0",
        }
    }

    /// Parse a manifest `specVersion` string.
    pub fn parse(s: &str) -> Result<Self, UnsupportedSpecVersion> {
        match s {
            "1.0" | "1" => Ok(SpecVersion::V1_0),
            other => Err(UnsupportedSpecVersion {
                found: String::from(other),
            }),
        }
    }
}

impl fmt::Display for SpecVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The manifest declared a `specVersion` this build cannot read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedSpecVersion {
    /// The raw value found in the manifest.
    pub found: String,
}

impl fmt::Display for UnsupportedSpecVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unsupported specVersion `{}`", self.found)?;
        write!(f, " (supported: ")?;
        for (i, v) in SpecVersion::ALL.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{v}")?;
        }
        write!(f, ")")
    }
}

impl std::error::Error for UnsupportedSpecVersion {}

impl Serialize for SpecVersion {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for SpecVersion {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        // Accept `1.0` (string) and, forgivingly, `1.0` (YAML float) so a
        // missing pair of quotes produces a validation error about content
        // rather than a confusing type error.
        let raw = SpecVersionRepr::deserialize(de)?;
        let text = match raw {
            SpecVersionRepr::Str(s) => s,
            SpecVersionRepr::Num(n) => alloc::format!("{n}"),
        };
        SpecVersion::parse(&text).map_err(serde::de::Error::custom)
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum SpecVersionRepr {
    Str(String),
    Num(f64),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_supported_versions() {
        assert_eq!(SpecVersion::parse("1.0").unwrap(), SpecVersion::V1_0);
        assert_eq!(SpecVersion::parse("1").unwrap(), SpecVersion::V1_0);
    }

    #[test]
    fn rejects_unknown_version_with_a_useful_message() {
        let err = SpecVersion::parse("2.0").unwrap_err();
        let msg = alloc::format!("{err}");
        assert!(msg.contains("2.0"), "{msg}");
        assert!(msg.contains("supported: 1.0"), "{msg}");
    }

    #[test]
    fn round_trips_through_json() {
        let json = serde_json::to_string(&SpecVersion::V1_0).unwrap();
        assert_eq!(json, "\"1.0\"");
        assert_eq!(
            serde_json::from_str::<SpecVersion>(&json).unwrap(),
            SpecVersion::V1_0
        );
    }

    #[test]
    fn tolerates_an_unquoted_yaml_number() {
        assert_eq!(
            serde_json::from_str::<SpecVersion>("1.0").unwrap(),
            SpecVersion::V1_0
        );
    }
}
