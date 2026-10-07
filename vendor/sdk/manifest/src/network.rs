//! Network selection: which chain, and how to reach it.

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use superquery_types::ChainFamily;
use url::Url;

/// The `network:` block of a manifest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NetworkConfig {
    /// Which chain family this project targets.
    pub family: ChainFamily,

    /// The chain identifier, as a string.
    ///
    /// A string rather than a number because it means different things per
    /// family: an EVM chain id (`"1"`), a Stellar network passphrase, a Solana
    /// genesis hash. Validation interprets it per family.
    pub chain_id: String,

    /// RPC endpoints, tried in order.
    #[serde(default)]
    pub endpoint: Endpoints,

    /// Optional dictionary endpoint used to skip empty block ranges.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dictionary: Option<Url>,

    /// Optional chain type/spec file for families that need one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chaintypes: Option<Utf8PathBuf>,
}

/// One or many RPC endpoints.
///
/// The manifest accepts a bare string or a list; both normalise to a list so
/// downstream code never branches on the spelling.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Endpoints {
    /// No endpoint declared in the manifest — supplied at run time instead.
    #[default]
    None,
    /// A single endpoint written as a bare string.
    One(Url),
    /// Several endpoints, tried in order.
    Many(Vec<Url>),
}

impl Endpoints {
    /// All declared endpoints, in declaration order.
    pub fn as_slice(&self) -> &[Url] {
        match self {
            Endpoints::None => &[],
            Endpoints::One(url) => std::slice::from_ref(url),
            Endpoints::Many(urls) => urls,
        }
    }

    /// Whether any endpoint was declared.
    pub fn is_empty(&self) -> bool {
        self.as_slice().is_empty()
    }
}
