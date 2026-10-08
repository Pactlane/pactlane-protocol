#!/usr/bin/env bash
# The same gate CI runs: formatting, lints, Wasm build, then all tests.
# Integration tests load the release Wasm, so the build must come first.
set -euo pipefail
cd "$(dirname "$0")/.."

cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
scripts/build.sh
cargo test --workspace
