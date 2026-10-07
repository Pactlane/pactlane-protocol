//! Loading policy around the SDK-owned manifest contract.
use crate::{manifest, ProjectManifest};
use std::path::{Path, PathBuf};

/// A validated project and its local inputs.
#[derive(Debug)]
pub struct LoadedProject {
    /// SDK manifest, without a second node wire representation.
    pub manifest: ProjectManifest,
    /// Canonical project directory.
    pub root: PathBuf,
    /// GraphQL schema source.
    pub schema: String,
}

impl LoadedProject {
    /// Read a project directory. Never search parents for an unrelated manifest.
    pub fn load(directory: impl AsRef<Path>) -> Result<Self, String> {
        let root = directory
            .as_ref()
            .canonicalize()
            .map_err(|e| format!("project directory: {e}"))?;
        let candidates: Vec<_> = ["project.yaml", "project.yml"]
            .into_iter()
            .map(|name| root.join(name))
            .filter(|p| p.is_file())
            .collect();
        if candidates.len() != 1 {
            return Err("project must contain exactly one project.yaml or project.yml".into());
        }
        let source =
            std::fs::read_to_string(&candidates[0]).map_err(|e| format!("manifest: {e}"))?;
        let manifest = manifest::from_str(&source, "project.yaml").map_err(|e| e.to_string())?;
        let report = manifest::validate(&manifest, None);
        if report.has_errors() {
            return Err(report
                .errors()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; "));
        }
        for ds in &manifest.data_sources {
            if ds.kind.as_str() != "evm/Runtime" {
                return Err(format!("unsupported datasource kind: {}", ds.kind));
            }
            for h in &ds.handlers {
                if !matches!(
                    h.kind.as_str(),
                    "evm/LogHandler" | "evm/TransactionHandler" | "evm/BlockHandler"
                ) {
                    return Err(format!("unsupported handler kind: {}", h.kind));
                }
            }
            for asset in ds.assets.values() {
                read_input(&root, asset.file.as_std_path())?;
            }
        }
        let schema = String::from_utf8(read_input(&root, manifest.schema.file.as_std_path())?)
            .map_err(|_| "schema must be UTF-8".to_string())?;
        Ok(Self {
            manifest,
            root,
            schema,
        })
    }
}

/// Read a project-owned file, refusing traversal and symlinks outside the bundle.
fn read_input(root: &Path, relative: &Path) -> Result<Vec<u8>, String> {
    if relative.is_absolute() {
        return Err("project input paths must be relative".into());
    }
    let path = root
        .join(relative)
        .canonicalize()
        .map_err(|e| format!("input {}: {e}", relative.display()))?;
    if !path.starts_with(root) {
        return Err(format!(
            "input {} escapes project directory",
            relative.display()
        ));
    }
    std::fs::read(path).map_err(|e| format!("input {}: {e}", relative.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn loads_real_sdk_template() {
        let p = LoadedProject::load("../../tests/fixtures/sdk-erc20").unwrap();
        assert_eq!(p.manifest.name, "erc20-transfers");
        assert_eq!(p.manifest.min_start_block(), 21_000_000);
        assert!(p.schema.contains("Transfer"));
    }
    #[test]
    fn rejects_future_manifest_version() {
        let yaml = include_str!("../../../tests/fixtures/sdk-erc20/project.yaml");
        assert!(manifest::from_str(&yaml.replace("\"1.0\"", "\"9.0\""), "test").is_err());
    }
    #[test]
    fn rejects_inputs_outside_project() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .canonicalize()
            .unwrap();
        assert!(read_input(&root, Path::new("../store/Cargo.toml")).is_err());
        assert!(read_input(&root, Path::new("/etc/passwd")).is_err());
    }
}
