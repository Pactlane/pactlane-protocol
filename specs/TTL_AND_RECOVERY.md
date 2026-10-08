# TTL and recovery

> **Status: placeholder.** Backlog #004 (expiry, TTL and recovery test coverage).

Soroban archives ledger entries whose TTL lapses. An archived escrow entry cannot
be rebuilt from Pactlane's database, so recovery has to be designed into the
contracts and runbooks. See the
[state archival docs](https://developers.stellar.org/docs/learn/fundamentals/contract-development/storage/state-archival).

## To document

- [ ] Which storage class (instance / persistent / temporary) each piece of job
      state uses, and why
- [ ] Who extends TTL on live jobs, when, and who pays for it
- [ ] Restoration procedure for an archived job entry, contract instance, and
      Wasm code entry
- [ ] What happens to a funded job whose entry archives before settlement, and the
      test proving funds are not lost
- [ ] Measured costs on testnet: CPU, memory, read/write bytes, fees, restoration
