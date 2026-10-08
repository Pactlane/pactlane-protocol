#!/usr/bin/env bash
# Deploy one contract to Stellar testnet and record it in deployments/testnet.json.
#
# Usage: STELLAR_ACCOUNT=<identity> scripts/deploy-testnet.sh <crate-name>
#
# STELLAR_ACCOUNT names a key known to the Stellar CLI. Create and fund one with:
#   stellar keys generate pactlane-deployer --network testnet --fund
set -euo pipefail
cd "$(dirname "$0")/.."

crate="${1:?usage: STELLAR_ACCOUNT=<identity> scripts/deploy-testnet.sh <crate-name>}"
: "${STELLAR_ACCOUNT:?set STELLAR_ACCOUNT to a Stellar CLI identity}"

if ! git diff --quiet HEAD -- contracts Cargo.toml Cargo.lock rust-toolchain.toml; then
  # The manifest records the git commit; a dirty tree would make it a lie.
  echo "Refusing to deploy: contract sources have uncommitted changes." >&2
  exit 1
fi

scripts/build.sh --package "$crate"
wasm="target/wasm32v1-none/release/${crate//-/_}.wasm"

contract_id="$(stellar contract deploy --wasm "$wasm" --source-account "$STELLAR_ACCOUNT" --network testnet)"
echo "Deployed $crate: $contract_id"

manifest=deployments/testnet.json
tmp="$(mktemp)"
jq --arg name "$crate" \
   --arg id "$contract_id" \
   --arg hash "$(sha256sum "$wasm" | cut -d' ' -f1)" \
   --arg commit "$(git rev-parse HEAD)" \
   --arg at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
   '.contracts[$name] = {contractId: $id, wasmSha256: $hash, gitCommit: $commit, deployedAt: $at}' \
   "$manifest" >"$tmp"
mv "$tmp" "$manifest"
echo "Recorded in $manifest"
