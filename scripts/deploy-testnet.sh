#!/usr/bin/env bash
# Deploy the commerce kernel and an evaluation policy bound to it on Stellar
# testnet, and record both in deployments/testnet.json.
#
# Usage: STELLAR_ACCOUNT=<identity> scripts/deploy-testnet.sh
#
#   STELLAR_ACCOUNT    a Stellar CLI identity that pays for deployment. Create
#                      and fund one with:
#                        stellar keys generate pactlane-deployer --network testnet --fund
#   POLICY_OWNER       who manages the policy's signers (default: the deployer)
#   TOKEN_CONTRACT_ID  the kernel's payment token (default: Circle's testnet
#                      USDC, as a Stellar Asset Contract)
set -euo pipefail
cd "$(dirname "$0")/.."

: "${STELLAR_ACCOUNT:?set STELLAR_ACCOUNT to a Stellar CLI identity}"
network=testnet
manifest=deployments/testnet.json

# Circle's testnet USDC issuer (home domain centre.io). The contract ID is
# derived from the asset, so it is fixed for this network.
usdc_asset="USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5"
token="${TOKEN_CONTRACT_ID:-$(stellar contract id asset --asset "$usdc_asset" --network "$network")}"
owner="${POLICY_OWNER:-$(stellar keys address "$STELLAR_ACCOUNT")}"

if ! git diff --quiet HEAD -- contracts Cargo.toml Cargo.lock rust-toolchain.toml; then
  # The manifest records the git commit; a dirty tree would make it a lie.
  echo "Refusing to deploy: contract sources have uncommitted changes." >&2
  exit 1
fi

scripts/build.sh >/dev/null
kernel_wasm=target/wasm32v1-none/release/pactlane_commerce.wasm
policy_wasm=target/wasm32v1-none/release/pactlane_evaluation_policy.wasm

deploy() {
  local wasm="$1"
  shift
  stellar contract deploy --wasm "$wasm" --source-account "$STELLAR_ACCOUNT" \
    --network "$network" -- "$@"
}

# The kernel's constructor probes the token, so a wrong token fails here.
kernel_id="$(deploy "$kernel_wasm" --token "$token")"
echo "Deployed pactlane-commerce: $kernel_id (token $token)"

# The policy's constructor probes the kernel, so a wrong kernel fails here.
policy_id="$(deploy "$policy_wasm" --owner "$owner" --kernel "$kernel_id")"
echo "Deployed pactlane-evaluation-policy: $policy_id (owner $owner)"

commit="$(git rev-parse HEAD)"
at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
built_with="$(jq -n --arg rustc "$(rustc --version)" --arg cli "$(stellar --version | head -1)" \
  '{rustc: $rustc, stellarCli: $cli}')"

# record <name> <contract id> <wasm> <constructor args as JSON>
record() {
  local tmp
  tmp="$(mktemp)"
  jq --arg name "$1" --arg id "$2" \
     --arg hash "$(sha256sum "$3" | cut -d' ' -f1)" \
     --arg commit "$commit" --arg at "$at" \
     --argjson args "$4" --argjson built "$built_with" \
     '.contracts[$name] = {contractId: $id, wasmSha256: $hash, gitCommit: $commit,
                           deployedAt: $at, constructorArgs: $args, builtWith: $built}' \
     "$manifest" >"$tmp"
  mv "$tmp" "$manifest"
}

record pactlane-commerce "$kernel_id" "$kernel_wasm" "$(jq -n --arg t "$token" '{token: $t}')"
record pactlane-evaluation-policy "$policy_id" "$policy_wasm" \
  "$(jq -n --arg o "$owner" --arg k "$kernel_id" '{owner: $o, kernel: $k}')"
echo "Recorded in $manifest. Next: add signers with"
echo "  stellar contract invoke --id $policy_id --source-account <owner> --network $network -- add_signer --signer <address>"
