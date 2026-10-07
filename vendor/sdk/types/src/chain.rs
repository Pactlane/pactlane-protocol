//! Chain families supported by the SDK.

use alloc::string::String;
use core::fmt;

use serde::{Deserialize, Serialize};

/// The family of a network, as declared by `network.family` in the manifest.
///
/// A family selects which datasource kinds, handler kinds and filters are
/// legal for a project, and which chain crate the CLI links for codegen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ChainFamily {
    /// Ethereum and EVM-compatible chains.
    Evm,
    /// Stellar / Soroban.
    Stellar,
    /// Solana.
    Solana,
}

impl ChainFamily {
    /// Every family this build knows about.
    pub const ALL: &'static [ChainFamily] =
        &[ChainFamily::Evm, ChainFamily::Stellar, ChainFamily::Solana];

    /// The manifest spelling of this family.
    pub const fn as_str(self) -> &'static str {
        match self {
            ChainFamily::Evm => "evm",
            ChainFamily::Stellar => "stellar",
            ChainFamily::Solana => "solana",
        }
    }

    /// The prefix used by this family's datasource and handler kinds,
    /// e.g. `evm/Runtime`, `evm/LogHandler`.
    pub const fn kind_prefix(self) -> &'static str {
        self.as_str()
    }

    /// Parse a manifest `network.family` value.
    pub fn parse(s: &str) -> Result<Self, UnknownChainFamily> {
        match s {
            "evm" | "ethereum" => Ok(ChainFamily::Evm),
            "stellar" => Ok(ChainFamily::Stellar),
            "solana" => Ok(ChainFamily::Solana),
            other => Err(UnknownChainFamily {
                found: String::from(other),
            }),
        }
    }

    /// Whether this family's developer surface is implemented in this build.
    ///
    /// Stellar and Solana ship as manifest-level placeholders until the EVM
    /// lifecycle is stable (Milestone 12).
    pub const fn is_implemented(self) -> bool {
        matches!(self, ChainFamily::Evm)
    }
}

impl fmt::Display for ChainFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// `network.family` named a chain family this build does not know.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownChainFamily {
    /// The raw value found in the manifest.
    pub found: String,
}

impl fmt::Display for UnknownChainFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown chain family `{}` (known: evm, stellar, solana)",
            self.found
        )
    }
}

impl std::error::Error for UnknownChainFamily {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_families_and_the_ethereum_alias() {
        assert_eq!(ChainFamily::parse("evm").unwrap(), ChainFamily::Evm);
        assert_eq!(ChainFamily::parse("ethereum").unwrap(), ChainFamily::Evm);
        assert_eq!(ChainFamily::parse("solana").unwrap(), ChainFamily::Solana);
        assert!(ChainFamily::parse("bitcoin").is_err());
    }

    #[test]
    fn serde_uses_the_manifest_spelling() {
        assert_eq!(serde_json::to_string(&ChainFamily::Evm).unwrap(), "\"evm\"");
    }
}
