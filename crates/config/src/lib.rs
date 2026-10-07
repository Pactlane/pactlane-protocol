//! # superquery-config
//!
//! Configuration for the indexer node: the CLI surface, environment variables and
//! their defaults.
//!
//! Kept in its own crate so the store, core and binary all agree on settings
//! without depending on each other.
//!
//! Upstream analogues: `node-core/src/configure/NodeConfig.ts` and
//! `node-core/src/db/db.module.ts`.

pub mod db;
pub mod node;

pub use db::{DbConfig, DbConfigError};
pub use node::{HistoricalMode, NodeConfig};

// The SDK owns these wire contracts.
pub use superquery_manifest::{self as manifest, ProjectManifest};
pub use superquery_types::{self as types};

pub mod project;
pub use project::LoadedProject;

pub mod endpoint;
