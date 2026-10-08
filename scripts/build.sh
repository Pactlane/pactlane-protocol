#!/usr/bin/env bash
# Build every contract to release Wasm and print each artifact's SHA-256.
# The hash is what deployments/*.json records and verify-deployment.ts checks.
set -euo pipefail
cd "$(dirname "$0")/.."

stellar contract build "$@"

echo
echo "Wasm artifacts:"
for wasm in target/wasm32v1-none/release/*.wasm; do
  printf '  %s  %s (%s bytes)\n' "$(sha256sum "$wasm" | cut -d' ' -f1)" "$wasm" "$(wc -c <"$wasm")"
done
