//! Project identity.

use alloc::string::{String, ToString};
use core::fmt;

use serde::{Deserialize, Serialize};

/// A project's stable identifier, derived from the manifest `name`.
///
/// Constrained to `[a-z0-9-]` because the same string becomes a directory
/// name, a Postgres schema name and a URL path segment downstream.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectId(String);

impl ProjectId {
    /// Maximum length, chosen to stay inside Postgres' 63-byte identifier
    /// limit once the node adds its own prefixes.
    pub const MAX_LEN: usize = 48;

    /// Validate and wrap a project name.
    pub fn new(name: impl Into<String>) -> Result<Self, InvalidProjectId> {
        let name = name.into();
        let invalid = |reason: &str| InvalidProjectId {
            found: name.clone(),
            reason: reason.to_string(),
        };

        if name.is_empty() {
            return Err(invalid("must not be empty"));
        }
        if name.len() > Self::MAX_LEN {
            return Err(invalid("must be at most 48 characters"));
        }
        if !name.starts_with(|c: char| c.is_ascii_lowercase()) {
            return Err(invalid("must start with a lowercase letter"));
        }
        if name.ends_with('-') {
            return Err(invalid("must not end with `-`"));
        }
        if !name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(invalid(
                "may only contain lowercase letters, digits and `-`",
            ));
        }
        Ok(Self(name))
    }

    /// The identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A project name that cannot be used as an identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidProjectId {
    /// The rejected name.
    pub found: String,
    /// Why it was rejected, phrased to complete "project name ...".
    pub reason: String,
}

impl fmt::Display for InvalidProjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid project name `{}`: {}", self.found, self.reason)
    }
}

impl std::error::Error for InvalidProjectId {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_conventional_name() {
        assert_eq!(
            ProjectId::new("erc20-transfers").unwrap().as_str(),
            "erc20-transfers"
        );
    }

    #[test]
    fn rejects_shapes_that_break_downstream_identifiers() {
        for bad in [
            "",
            "Erc20",
            "1st",
            "has_underscore",
            "trailing-",
            "has space",
        ] {
            assert!(
                ProjectId::new(bad).is_err(),
                "expected `{bad}` to be rejected"
            );
        }
    }

    #[test]
    fn rejects_names_longer_than_the_postgres_budget() {
        let long = "a".repeat(ProjectId::MAX_LEN + 1);
        assert!(ProjectId::new(long).is_err());
    }
}
