# deployments

One manifest per network, holding each deployed contract's ID, Wasm SHA-256,
source commit and deploy time. Consumers check their configured contract IDs
against these manifests at startup.

- `testnet.json`: written by `scripts/deploy-testnet.sh`, checked by
  `scripts/verify-deployment.ts`.
- `mainnet.json`: **deliberately absent.** It is created only after an independent
  security review (ADR-010). Its absence is the signal that nothing is released
  for mainnet. Never add it as a placeholder.
