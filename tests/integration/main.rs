mod archival;
mod commerce;
mod hello_world;

use std::path::PathBuf;

/// Reads a contract's release Wasm from the workspace target directory.
///
/// Read at runtime rather than with `contractimport!` so that `cargo check`
/// and the unit tests still work on a fresh clone with no Wasm built.
pub fn release_wasm(crate_name: &str) -> Vec<u8> {
    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target"));
    let path = target_dir
        .join("wasm32v1-none/release")
        .join(format!("{}.wasm", crate_name.replace('-', "_")));

    std::fs::read(&path).unwrap_or_else(|err| {
        panic!(
            "cannot read {}: {err}. Run scripts/build.sh before the integration tests.",
            path.display()
        )
    })
}
