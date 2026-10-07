//! The SuperQuery project manifest: `project.yaml`.
//!
//! This crate is the single definition of the manifest format. The CLI, the
//! node and any future tooling all parse through it — there is deliberately no
//! second `ProjectManifest` anywhere in the platform (see `docs/spec/manifest-v1.md`).
//!
//! Two layers, kept apart on purpose:
//!
//! 1. [`reader`] turns YAML bytes into a [`ProjectManifest`]. Syntax only.
//! 2. [`validate`] answers whether that manifest describes a project that
//!    could actually run: known chain family, handlers that exist, assets on
//!    disk, filters legal for the family.
//!
//! Parsing succeeds for plenty of manifests that validation rejects. That
//! split is what lets `superquery validate` point at an exact field instead of
//! failing with a serde type error.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod datasource;
pub mod error;
pub mod filter;
pub mod handler;
pub mod network;
pub mod project;
pub mod reader;
pub mod validate;

pub use datasource::{DataSource, DataSourceKind};
pub use error::{ManifestError, ManifestResult};
pub use filter::{HandlerFilter, LogFilter, TransactionFilter};
pub use handler::{Handler, HandlerKind};
pub use network::{Endpoints, NetworkConfig};
pub use project::{ProjectManifest, SchemaRef};
pub use reader::{from_path, from_str};
pub use validate::{Diagnostic, Severity, ValidationReport, validate};

/// Conventional file name for a project manifest.
pub const MANIFEST_FILE_NAME: &str = "project.yaml";

/// Alternate file name accepted by the reader, for developers who prefer it.
pub const MANIFEST_FILE_NAME_ALT: &str = "project.yml";
