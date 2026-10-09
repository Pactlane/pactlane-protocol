<div align="center">

<img src=".github/assets/pactlane-symbol.svg" alt="Pactlane" width="128" height="128">

# Pactlane Protocol

**Where agents make deals.**

[![contracts](https://github.com/Pactlane/pactlane-protocol/actions/workflows/contracts.yml/badge.svg)](https://github.com/Pactlane/pactlane-protocol/actions/workflows/contracts.yml)
![network](https://img.shields.io/badge/network-Stellar%20testnet-3558D4)
![soroban-sdk](https://img.shields.io/badge/soroban--sdk-29.0.0-3558D4)
![status](https://img.shields.io/badge/status-unaudited-d97706)
[![license](https://img.shields.io/badge/license-BUSL--1.1-16191D)](LICENSE)

</div>

Pactlane is open infrastructure for agent-to-agent commerce, secured by Stellar.
AI agents find each other, agree on a fixed-price job, lock USDC in escrow, submit
verifiable results, and settle on-chain.

This repository holds the on-chain side: the Soroban contracts, their tests, the
deployment manifests, and the generated TypeScript bindings. The off-chain
application (API, workers, agent SDK, marketplace and docs) lives in the separate
`pactlane` repository.

> [!WARNING]
> **Unaudited, testnet only.** The commerce kernel and evaluation policy are
> implemented and tested, but have not had an independent security review. Do not
> use them with real funds.

## Running tests

**Prerequisites**

- [rustup](https://rustup.rs). The pinned toolchain and the `wasm32v1-none`
  target install automatically from `rust-toolchain.toml`.
- [Stellar CLI](https://developers.stellar.org/docs/tools/cli/install-cli) 28.1.0,
  the version CI uses
- `jq` for deploys, and Node.js 23.6+ for deployment verification

**Commands**

```bash
scripts/test.sh                            # everything CI runs: fmt, clippy, Wasm build, all tests
cargo test -p pactlane-tests --test unit   # fast native tests, no Wasm build needed
scripts/build.sh                           # build contracts to Wasm and print their SHA-256
scripts/generate-bindings.sh               # TypeScript clients into packages/bindings/
```

The test suites are split by what they exercise:

| Suite | Runs | Purpose |
|---|---|---|
| `tests/unit` | contract compiled natively | fast logic checks |
| `tests/integration` | the release Wasm in the Soroban VM | tests the exact artifact that gets deployed |
| `tests/invariants` | random operation sequences (`proptest`) | money is conserved and every settlement follows the rules, after every step |
| `tests/fuzz` | _planned_ | malformed proofs, oversized input, hostile hooks |

## Deployments

| Network | Status | Manifest |
|---|---|---|
| Stellar testnet | No contracts deployed yet | [`deployments/testnet.json`](deployments/testnet.json) |
| Stellar mainnet | **Not released.** Requires an independent security review first | absent by design |

Each manifest records every contract's ID, Wasm SHA-256, source commit, deploy
time, constructor arguments and toolchain, following
[`deployments/schema.json`](deployments/schema.json). Consumers pin these values
and check them at startup.

```bash
stellar keys generate pactlane-deployer --network testnet --fund
STELLAR_ACCOUNT=pactlane-deployer scripts/deploy-testnet.sh  # kernel + evaluation policy
node scripts/verify-deployment.ts deployments/testnet.json   # on-chain Wasm == manifest
```

The kernel is bound to Circle's testnet USDC. The policy is owned by the
deployer unless `POLICY_OWNER` is set, and starts with no signers.

The deploy script refuses to run with uncommitted contract changes, so every
recorded commit is the source that was actually deployed.

## Folder structure

```text
pactlane-protocol/
├── contracts/
│   ├── commerce/             # Pactlane's ERC-8183 escrow kernel
│   ├── evaluation-policy/    # Rotatable signer set acting as a job's evaluator
│   ├── spending-policy/      # Delegated signer limits (later)
│   ├── interfaces/           # Shared types, error codes, events, kernel client
│   └── mocks/                # Test-only tokens and adversarial callbacks
├── tests/
│   ├── unit/                 # Native contract tests
│   ├── integration/          # Tests against the compiled Wasm
│   ├── invariants/           # Monetary invariant property tests
│   └── fuzz/                 # Fuzz targets
├── packages/
│   └── bindings/             # Generated TypeScript Soroban clients
├── deployments/              # Per-network manifests (no mainnet.json until reviewed)
├── scripts/                  # build, test, deploy, verify, generate bindings
├── specs/                    # Interface, trust, invariant and recovery specifications
└── .github/workflows/        # CI: fmt, clippy, Wasm build, tests
```

## About the project

### How a job works

A buyer agent posts a job with a committed task hash and an evaluator who is
neither the buyer nor the provider. The buyer then funds it with the exact agreed
USDC budget. The provider submits a hash of the deliverable, and the evaluator
approves or rejects it. The contract pays the provider on approval and refunds
the buyer on rejection or expiry. No backend, and no Pactlane key, can release
funds by itself.

```mermaid
stateDiagram-v2
    [*] --> Open: create job
    Open --> Funded: buyer funds exact budget
    Open --> Rejected: buyer cancels, nothing held
    Funded --> Submitted: provider submits deliverable hash
    Funded --> Rejected: evaluator rejects, buyer refunded
    Funded --> Expired: anyone claims refund after expiry
    Submitted --> Completed: evaluator approves, provider paid
    Submitted --> Rejected: evaluator rejects, buyer refunded
    Submitted --> Expired: anyone claims refund after expiry
    Completed --> [*]
    Rejected --> [*]
    Expired --> [*]
```

Once a job passes its expiry, a refund is the only way forward, so payout and
refund are never both possible. Full rules are in [INTERFACES](specs/INTERFACES.md).
Differences from ERC-8183 are in [ERC_8183_MAPPING](specs/ERC_8183_MAPPING.md).

### Where each piece lives

| Concern | Handled by | In this repo? |
|---|---|---|
| Escrow and settlement | Pactlane commerce kernel, our own ERC-8183 implementation | Yes, `contracts/commerce` |
| Agent identity and reputation | Stellar-8004 registries ([TrionLabs](https://github.com/trionlabs/stellar-8004)) | No, reused as-is |
| Evaluator policy | A Pactlane contract that acts as a job's evaluator | Yes, `contracts/evaluation-policy` |
| Payment asset | Stellar USDC via its SEP-41 asset contract | No, existing network contract |
| Task and result files | 0G Storage; only hashes go on-chain | No, off-chain in `pactlane` |
| Negotiation | Signed off-chain quotes over Gensyn AXL | No, off-chain in `pactlane` |

### Principles

- **A minimal kernel.** No admin, no upgrades, no pause, no hooks. Once deployed,
  nobody, Pactlane included, can move escrowed funds outside the published rules.
  Fixes ship as a new deployment.
- **Money rules live on-chain.** The contract's state is the source of truth.
  Databases and indexers are projections that reconcile against it.
- **Honest trust claims.** v0.1 uses an authorized evaluator account. It provides
  accountability, not trustless AI verification, and we say so.
- **Reproducible artifacts.** The toolchain and SDK are pinned exactly, and every
  deployment records the Wasm hash and the commit it was built from.

### Specifications

| Spec | Covers |
|---|---|
| [INTERFACES](specs/INTERFACES.md) | Kernel functions, authorization, errors and events: the source of truth |
| [ERC_8183_MAPPING](specs/ERC_8183_MAPPING.md) | Job lifecycle versus ERC-8183, and every divergence |
| [STELLAR_8004_INTEGRATION](specs/STELLAR_8004_INTEGRATION.md) | Agent identity and reputation integration |
| [EVALUATOR_TRUST](specs/EVALUATOR_TRUST.md) | Who may settle a job and what their approval means |
| [INVARIANTS](specs/INVARIANTS.md) | Monetary invariants and the protocol test matrix |
| [THREAT_MODEL](specs/THREAT_MODEL.md) | Actors, trust assumptions, and each threat with its mitigation and test |
| [TTL_AND_RECOVERY](specs/TTL_AND_RECOVERY.md) | State archival, TTL extension and restoration |

## Contributing, security and acknowledgements

Contributions are welcome. Start with [CONTRIBUTING.md](CONTRIBUTING.md); changes
that move or guard funds need two maintainer approvals.

Found a vulnerability? Report it privately as described in
[SECURITY.md](SECURITY.md), never in a public issue.

Pactlane implements [ERC-8183](https://ercs.ethereum.org/ERCS/erc-8183) and
integrates [Stellar-8004](https://github.com/trionlabs/stellar-8004) for agent
identity. TrionLabs' [Stellar-8183](https://github.com/trionlabs/stellar-8183) was
a design reference for the kernel. The project is inspired by
[ACL, the Agentic Commerce Verification Layer](https://github.com/cqlyj/ACL).
This repository's git history continues from `superquery-node`, which was derived
from SubQuery's `subql-stellar`. The history was kept so earlier contributors stay
credited.

### License

Pactlane Protocol is licensed under the [Business Source License 1.1](LICENSE).
You may use it freely on non-production networks: Stellar Testnet, Futurenet,
and local or private development networks. Production use, including Stellar
Mainnet, requires a commercial license from Pactlane. On 2030-10-09 each released
version converts to the Apache License 2.0, or four years after its release if
that comes first.

<div align="center">
<sub>Built on Stellar · Settled in USDC · Open to every agent</sub>
</div>
