//! Handlers: the mapping functions a datasource dispatches to.

use serde::{Deserialize, Serialize};
use superquery_types::ChainFamily;

use crate::filter::HandlerFilter;

/// One entry in a datasource's `handlers:` list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Handler {
    /// The exported mapping function name, e.g. `handle_transfer`.
    pub handler: String,

    /// What kind of chain item this handler receives, e.g. `evm/LogHandler`.
    pub kind: HandlerKind,

    /// Optional filter narrowing what reaches the handler.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<HandlerFilter>,
}

/// A handler kind, spelled `<family>/<Kind>Handler` in the manifest.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HandlerKind(String);

impl HandlerKind {
    /// Wrap a raw handler kind string.
    pub fn new(kind: impl Into<String>) -> Self {
        Self(kind.into())
    }

    /// The full kind string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The family prefix, e.g. `evm`.
    pub fn family_prefix(&self) -> Option<&str> {
        self.0.split_once('/').map(|(prefix, _)| prefix)
    }

    /// The part after the slash, e.g. `LogHandler`.
    pub fn suffix(&self) -> Option<&str> {
        self.0.split_once('/').map(|(_, suffix)| suffix)
    }

    /// Whether this kind belongs to `family`.
    pub fn belongs_to(&self, family: ChainFamily) -> bool {
        self.family_prefix() == Some(family.kind_prefix())
    }
}

impl std::fmt::Display for HandlerKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
