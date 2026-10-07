//! Reading manifests from strings and from disk.
//!
//! Syntax only. A manifest that parses here may still be rejected by
//! [`crate::validate`] — that separation is what makes error messages good.

use camino::{Utf8Path, Utf8PathBuf};
use miette::{NamedSource, SourceSpan};

use crate::error::{ManifestError, ManifestResult};
use crate::project::ProjectManifest;
use crate::{MANIFEST_FILE_NAME, MANIFEST_FILE_NAME_ALT};

/// Parse a manifest from YAML source.
///
/// `name` labels the source in diagnostics; pass the file path when you have
/// one, or something like `"<stdin>"` when you do not.
pub fn from_str(source: &str, name: &str) -> ManifestResult<ProjectManifest> {
    serde_yaml_ng::from_str(source).map_err(|err| {
        // serde_yaml_ng reports a line/column for most failures. Turn it into
        // a byte span so miette can underline the offending line.
        let span = err.location().map(|loc| {
            let start = byte_offset(source, loc.line(), loc.column());
            let len = line_len_from(source, start);
            SourceSpan::from((start, len))
        });
        ManifestError::Yaml {
            message: err.to_string(),
            src: NamedSource::new(name, source.to_owned()).with_language("yaml"),
            span,
        }
    })
}

/// Read and parse the manifest at an exact file path.
pub fn from_path(path: &Utf8Path) -> ManifestResult<ProjectManifest> {
    let source = std::fs::read_to_string(path).map_err(|source| ManifestError::Unreadable {
        path: path.to_owned(),
        source,
    })?;
    from_str(&source, path.as_str())
}

/// Find the manifest for `dir`, searching upward through parent directories.
///
/// Mirrors how `cargo` finds `Cargo.toml`, so running a command from a
/// subdirectory of a project does the obvious thing.
pub fn discover(dir: &Utf8Path) -> ManifestResult<Utf8PathBuf> {
    for candidate_dir in dir.ancestors() {
        for name in [MANIFEST_FILE_NAME, MANIFEST_FILE_NAME_ALT] {
            let candidate = candidate_dir.join(name);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    Err(ManifestError::NotFound {
        dir: dir.to_owned(),
    })
}

/// Byte offset of a 1-based line/column pair.
fn byte_offset(source: &str, line: usize, column: usize) -> usize {
    let mut offset = 0;
    for (i, l) in source.lines().enumerate() {
        if i + 1 == line {
            // Clamp: the reported column can sit one past the last character.
            return offset + column.saturating_sub(1).min(l.len());
        }
        offset += l.len() + 1;
    }
    source.len()
}

/// Length of the line containing `start`, so a span covers the whole line.
fn line_len_from(source: &str, start: usize) -> usize {
    source[start..]
        .find('\n')
        .unwrap_or(source.len() - start)
        .max(1)
}
