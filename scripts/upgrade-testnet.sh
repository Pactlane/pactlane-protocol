#!/usr/bin/env bash
# Upgrade the deployed testnet contracts in place: same contract IDs, new code.
# Builds the current commit, uploads each Wasm whose hash differs from the
# manifest, calls `upgrade`, and records the new hash and commit.
#
# Usage: STELLAR_ACCOUNT=<identity> scripts/upgrade-testnet.sh [kernel|policy|all]
#
#   STELLAR_ACCOUNT  the kernel's admin / the policy's owner. It signs the
#                    upgrade and pays the fees.
set -euo pipefail
cd "$(dirname "$0")/.."

: "${STELLAR_ACCOUNT:?set STELLAR_ACCOUNT to the admin's Stellar CLI identity}"
which="${1:-all}"
network=testnet
manifest=deployments/testnet.json

if ! git diff --quiet HEAD -- contracts Cargo.toml Cargo.lock rust-toolchain.toml; then
  # The manifest records the git commit; a dirty tree would make it a lie.
  echo "Refusing to upgrade: contract sources have uncommitted changes." >&2
  exit 1
fi

scripts/build.sh >/dev/null
commit="$(git rev-parse HEAD)"
at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
built_with="$(jq -n --arg rustc "$(rustc --version)" --arg cli "$(stellar --version | head -1)" \
  '{rustc: $rustc, stellarCli: $cli}')"

# upgrade <manifest name> <wasm>
upgrade() {
  local name="$1" wasm="$2" id recorded hash tmp
  id="$(jq -r --arg n "$name" '.contracts[$n].contractId // empty' "$manifest")"
  if [ -z "$id" ]; then
    echo "$name is not deployed; run scripts/deploy-testnet.sh first." >&2
    exit 1
  fi
  recorded="$(jq -r --arg n "$name" '.contracts[$n].wasmSha256' "$manifest")"
  hash="$(sha256sum "$wasm" | cut -d' ' -f1)"
  if [ "$hash" = "$recorded" ]; then
    echo "$name: already running $hash, nothing to do"
    return
  fi

  stellar contract upload --wasm "$wasm" --source-account "$STELLAR_ACCOUNT" \
    --network "$network" >/dev/null
  stellar contract invoke --id "$id" --source-account "$STELLAR_ACCOUNT" \
    --network "$network" -- upgrade --new_wasm_hash "$hash" >/dev/null
  echo "Upgraded $name $id: $recorded -> $hash"

  tmp="$(mktemp)"
  jq --arg n "$name" --arg hash "$hash" --arg commit "$commit" --arg at "$at" \
     --argjson built "$built_with" \
     '.contracts[$n] += {wasmSha256: $hash, gitCommit: $commit, upgradedAt: $at, builtWith: $built}' \
     "$manifest" >"$tmp"
  mv "$tmp" "$manifest"
}

case "$which" in kernel | policy | all) ;; *) echo "usage: $0 [kernel|policy|all]" >&2; exit 2 ;; esac
if [ "$which" != policy ]; then
  upgrade pactlane-commerce target/wasm32v1-none/release/pactlane_commerce.wasm
fi
if [ "$which" != kernel ]; then
  upgrade pactlane-evaluation-policy target/wasm32v1-none/release/pactlane_evaluation_policy.wasm
fi
echo "Recorded in $manifest. Check with: node scripts/verify-deployment.ts"
