# pactlane-protocol

Soroban contracts, tests, deployment manifests and generated TypeScript bindings
for **Pactlane**, open infrastructure for agent-to-agent commerce, secured by
Stellar.

The off-chain application (API, workers, SDK, web, docs) lives in the separate
`pactlane` repository. That repository consumes this one only through versioned
deployment manifests and bindings.

> **Status: scaffold.** The only contract is a placeholder `hello-world` that
> exercises the build → test → deploy → bindings pipeline. Nothing here is audited.
> Testnet only. There is no mainnet deployment.

## Layout

```text
contracts/
  hello-world/        placeholder; removed once a real contract covers the pipeline
  commerce/           ERC-8183 escrow kernel, only if upstream must be customized
  evaluation-policy/  evaluator allowlist / proof policy hook
  spending-policy/    delegated signer limits (later)
  interfaces/         typed Rust interfaces and hook structures
  mocks/              test-only tokens and adversarial callbacks
tests/                unit (native), integration (compiled Wasm), invariants, fuzz
packages/bindings/    generated TypeScript clients
deployments/          per-network manifests: contract IDs, Wasm hashes, commits
scripts/              build, test, deploy, verify, generate bindings
specs/                interface, trust, invariant and recovery specifications
```

## Prerequisites

- [rustup](https://rustup.rs). The toolchain and the `wasm32v1-none` target are
  pinned in `rust-toolchain.toml` and installed automatically.
- [Stellar CLI](https://developers.stellar.org/docs/tools/cli/install-cli) 28.1.0,
  the version CI uses
- `jq` (deploy script), Node.js 23.6+ (verify script)

## Commands

```bash
scripts/build.sh              # build all contracts to Wasm, print SHA-256s
scripts/test.sh               # fmt, clippy, build, all tests (what CI runs)
cargo test -p pactlane-tests --test unit   # fast native tests, no Wasm build
scripts/generate-bindings.sh  # TypeScript clients into packages/bindings/
```

Deploy to testnet and record the result in `deployments/testnet.json`:

```bash
stellar keys generate pactlane-deployer --network testnet --fund
STELLAR_ACCOUNT=pactlane-deployer scripts/deploy-testnet.sh hello-world
node scripts/verify-deployment.ts deployments/testnet.json
```

## Network

`soroban-sdk` is pinned to 29.0.0 because the SDK major version tracks the Stellar
protocol, and testnet runs protocol 29. Upgrade the SDK only together with the
network.

## History

This repository's git history continues from `superquery-node`, which was derived
from SubQuery's `subql-stellar`. The history is kept so earlier contributors stay
credited. None of that code is in the current tree.

## Contributing and security

See [CONTRIBUTING.md](CONTRIBUTING.md). Report vulnerabilities privately as
described in [SECURITY.md](SECURITY.md), never in a public issue.
