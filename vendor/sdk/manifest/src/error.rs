//! Manifest errors.
//!
//! Every variant carries enough context to point a developer at the exact
//! place in their project that is wrong. Syntax errors carry a source span;
//! semantic failures carry the field path.

use camino::Utf8PathBuf;
use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

/// Result alias for manifest operations.
pub type ManifestResult<T> = Result<T, ManifestError>;

/// Something went wrong reading or interpreting a manifest.
#[derive(Debug, Error, Diagnostic)]
#[non_exhaustive]
pub enum ManifestError {
    /// The manifest file could not be read from disk.
    #[error("could not read manifest at `{path}`")]
    #[diagnostic(
        code(superquery::manifest::unreadable),
        help("run `superquery init` to create a project, or pass --manifest to point at one")
    )]
    Unreadable {
        /// The path that failed.
        path: Utf8PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// No `project.yaml` was found while searching upward from a directory.
    #[error("no `project.yaml` found in `{dir}` or any parent directory")]
    #[diagnostic(
        code(superquery::manifest::not_found),
        help("run this command inside a SuperQuery project, or create one with `superquery init`")
    )]
    NotFound {
        /// Where the search started.
        dir: Utf8PathBuf,
    },

    /// The YAML did not parse, or did not match the manifest shape.
    #[error("{message}")]
    #[diagnostic(code(superquery::manifest::invalid_yaml))]
    Yaml {
        /// The serde message, already human-readable.
        message: String,
        /// The manifest source, for span rendering.
        #[source_code]
        src: NamedSource<String>,
        /// Where in the source the problem is, when the parser reported it.
        #[label("here")]
        span: Option<SourceSpan>,
    },

    /// The manifest parsed but does not describe a runnable project.
    #[error("manifest failed validation with {count} error(s)")]
    #[diagnostic(code(superquery::manifest::invalid))]
    Invalid {
        /// How many error-severity diagnostics were produced.
        count: usize,
        /// The individual problems.
        #[related]
        diagnostics: Vec<crate::validate::Diagnostic>,
    },
}
