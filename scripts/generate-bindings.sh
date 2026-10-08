#!/usr/bin/env bash
# Generate a TypeScript client for each built contract into packages/bindings/.
# Generated from the local Wasm, so run scripts/build.sh first.
set -euo pipefail
cd "$(dirname "$0")/.."

shopt -s nullglob
wasms=(target/wasm32v1-none/release/*.wasm)
if [ ${#wasms[@]} -eq 0 ]; then
  echo "No Wasm found. Run scripts/build.sh first." >&2
  exit 1
fi

for wasm in "${wasms[@]}"; do
  name="$(basename "$wasm" .wasm | tr '_' '-')"
  echo "Generating bindings for $name"
  stellar contract bindings typescript \
    --wasm "$wasm" \
    --output-dir "packages/bindings/$name" \
    --overwrite
done
