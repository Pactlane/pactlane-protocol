//! Mapping ABI versioning.
//!
//! The mapping ABI is the boundary between a compiled `mapping.wasm` and the
//! node runtime that hosts it. The guest stamps the version it was built
//! against into `build.json`; the node refuses a bundle it cannot host, with a
//! clear message rather than a trap deep inside a host call.
//!
//! See `docs/spec/mapping-abi-v1.md`.

use core::fmt;

use serde::{Deserialize, Serialize};

/// The mapping ABI version this SDK build implements.
pub const MAPPING_ABI_VERSION: MappingAbiVersion = MappingAbiVersion(1);

/// A mapping ABI version number.
///
/// Monotonic and deliberately not semver: a host either implements a version
/// or it does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MappingAbiVersion(pub u32);

impl MappingAbiVersion {
    /// The raw number, as written to `build.json`.
    pub const fn get(self) -> u32 {
        self.0
    }

    /// Whether a host implementing `self` can run a guest built against
    /// `guest`. Hosts are backward compatible within a major line; a guest
    /// from the future is always rejected.
    pub const fn can_host(self, guest: MappingAbiVersion) -> bool {
        guest.0 <= self.0
    }
}

impl fmt::Display for MappingAbiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_host_runs_its_own_and_older_guests() {
        let host = MappingAbiVersion(2);
        assert!(host.can_host(MappingAbiVersion(1)));
        assert!(host.can_host(MappingAbiVersion(2)));
        assert!(!host.can_host(MappingAbiVersion(3)));
    }

    #[test]
    fn version_serializes_as_a_bare_number() {
        assert_eq!(serde_json::to_string(&MAPPING_ABI_VERSION).unwrap(), "1");
    }
}
