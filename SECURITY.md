# Security policy

## Status

These contracts are **unaudited and testnet-only**. Do not send mainnet funds to
any address associated with this repository. A mainnet release requires an
independent security review first. Until then `deployments/mainnet.json` does not
exist.

## Reporting a vulnerability

Report privately through GitHub: **Security → Report a vulnerability** on this
repository. Do not open a public issue or pull request for a suspected
vulnerability.

Please include the affected contract and commit, the impact (e.g. funds locked,
stolen, or settled twice), and steps or a test that reproduces it.

## Scope

In scope: everything under `contracts/`, the deployment scripts, and the
deployment manifests.

Out of scope: upstream contracts this project integrates but does not modify
(Stellar-8183, Stellar-8004, the USDC Stellar Asset Contract). Report those to
their maintainers. Report issues in the off-chain application to the `pactlane`
repository.
