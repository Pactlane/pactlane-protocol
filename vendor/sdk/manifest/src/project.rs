//! The top-level manifest document.

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use superquery_types::SpecVersion;

use crate::datasource::DataSource;
use crate::network::NetworkConfig;

/// A parsed `project.yaml`.
///
/// Field names are camelCase on the wire to match the format developers
/// already know from SubQuery projects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectManifest {
    /// Manifest format version. Explicit from day one so the reader can
    /// refuse a future format instead of silently mis-parsing it.
    pub spec_version: SpecVersion,

    /// Project name. Becomes the project id, so it must be a valid
    /// [`superquery_types::ProjectId`] — checked during validation, not parsing.
    pub name: String,

    /// The developer's own version string for this project.
    pub version: String,

    /// Optional human description, surfaced by tooling and the explorer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Optional SPDX license identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    /// Which network this project indexes.
    pub network: NetworkConfig,

    /// Where the GraphQL entity schema lives.
    pub schema: SchemaRef,

    /// Default start height, used by any datasource that does not set its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_block: Option<u64>,

    /// The contracts, accounts or blocks this project reacts to.
    pub data_sources: Vec<DataSource>,
}

impl ProjectManifest {
    /// The effective start height for a datasource: its own, else the
    /// project-level default, else genesis.
    pub fn effective_start_block(&self, ds: &DataSource) -> u64 {
        ds.start_block.or(self.start_block).unwrap_or(0)
    }

    /// The lowest height this project needs to begin indexing from.
    pub fn min_start_block(&self) -> u64 {
        self.data_sources
            .iter()
            .map(|ds| self.effective_start_block(ds))
            .min()
            .unwrap_or_else(|| self.start_block.unwrap_or(0))
    }

    /// Every handler function name referenced by the manifest, in order.
    ///
    /// The CLI checks these against the symbols the mapping crate exports.
    pub fn handler_names(&self) -> impl Iterator<Item = &str> {
        self.data_sources
            .iter()
            .flat_map(|ds| ds.handlers.iter())
            .map(|h| h.handler.as_str())
    }
}

/// A reference to the project's GraphQL schema file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SchemaRef {
    /// Path to `schema.graphql`, relative to the manifest.
    pub file: Utf8PathBuf,
}
