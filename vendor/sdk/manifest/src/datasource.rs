//! Datasources: what a project watches.

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use superquery_types::ChainFamily;

use crate::handler::Handler;

/// One entry in the manifest's `dataSources:` list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DataSource {
    /// The datasource kind, e.g. `evm/Runtime`.
    pub kind: DataSourceKind,

    /// Optional name, used in diagnostics and required to be unique if set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Height this datasource starts at, overriding the project default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_block: Option<u64>,

    /// Height this datasource stops at, for finite sources.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_block: Option<u64>,

    /// Family-specific options, e.g. `address` and `abi` for EVM.
    #[serde(default)]
    pub options: DataSourceOptions,

    /// Named asset files (ABIs, IDLs) this datasource needs.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub assets: BTreeMap<String, AssetRef>,

    /// The handlers this datasource dispatches to.
    pub handlers: Vec<Handler>,
}

impl DataSource {
    /// A stable label for diagnostics: the declared name, else the kind.
    pub fn label(&self) -> &str {
        self.name.as_deref().unwrap_or_else(|| self.kind.as_str())
    }
}

/// A datasource kind, spelled `<family>/<Kind>` in the manifest.
///
/// Parsed rather than enumerated so a chain crate can introduce a kind without
/// a breaking change to this crate; [`crate::validate`] is what decides
/// whether a given kind is legal for the declared family.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DataSourceKind(String);

impl DataSourceKind {
    /// Wrap a raw kind string.
    pub fn new(kind: impl Into<String>) -> Self {
        Self(kind.into())
    }

    /// The full kind string, e.g. `evm/Runtime`.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The family prefix, e.g. `evm`, if the kind is well-formed.
    pub fn family_prefix(&self) -> Option<&str> {
        self.0.split_once('/').map(|(prefix, _)| prefix)
    }

    /// The part after the slash, e.g. `Runtime`.
    pub fn suffix(&self) -> Option<&str> {
        self.0.split_once('/').map(|(_, suffix)| suffix)
    }

    /// Whether this kind belongs to `family`.
    pub fn belongs_to(&self, family: ChainFamily) -> bool {
        self.family_prefix() == Some(family.kind_prefix())
    }
}

impl std::fmt::Display for DataSourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Family-specific datasource options.
///
/// Held untyped at this layer so the manifest crate stays chain-agnostic. The
/// chain crates interpret and validate their own keys.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DataSourceOptions(pub BTreeMap<String, serde_json::Value>);

impl DataSourceOptions {
    /// Read a string option, e.g. `address`.
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(serde_json::Value::as_str)
    }

    /// Whether an option is present.
    pub fn contains(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    /// Option keys, sorted.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }
}

/// A file this datasource depends on, such as a contract ABI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetRef {
    /// Path to the asset, relative to the manifest.
    pub file: Utf8PathBuf,
}
