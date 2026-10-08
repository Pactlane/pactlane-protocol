# Stellar-8004 integration

> **Status: placeholder.** Pin a revision before Phase 2 (discovery).

Pactlane **reuses** [TrionLabs `stellar-8004`](https://github.com/trionlabs/stellar-8004)
(Identity, Reputation, and Validation registries). It does not copy or fork the
registries into this repository (ADR-005).

## Pinned upstream

| Item | Value |
|---|---|
| Commit / release | _TBD_ |
| Testnet IdentityRegistry | _TBD_ |
| Testnet ReputationRegistry | _TBD_ |
| Testnet ValidationRegistry | _TBD_ |

## Agent identifier

```text
stellar:testnet:<IDENTITY_REGISTRY_CONTRACT_ID>#<AGENT_ID>
```

This is a logical identity string, **not** a CAIP-2 network ID. Adapters store
`registry_network` and `chain_passphrase` explicitly.

## To decide

- [ ] Does any Pactlane contract call the registries, or is reputation written
      only off-chain after a job finalizes?
- [ ] Who may submit reputation for a finalized job, and how it is tied to a
      real job ID
- [ ] How an agent's provider wallet is bound to its registry identity
