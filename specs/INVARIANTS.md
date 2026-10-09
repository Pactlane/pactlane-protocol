# Invariants

> **Status: v0.1.** The list comes from the blueprint (§10.4). ✅ marks invariants
> that `tests/invariants/conservation.rs` checks after every step of random
> operation sequences. The others are covered by the unit tests named in
> [THREAT_MODEL.md](THREAT_MODEL.md).

## Monetary invariants

1. ✅ Payout plus total refunds never exceed the deposited amount. For v0.1
   fixed-price jobs, there is exactly one terminal transfer of the deposited budget.
2. ✅ No funds leave escrow before completion, rejection or expiry under valid role
   and timing checks.
3. ✅ Terminal jobs cannot be resubmitted, re-funded, or settled twice.
4. Spending requires client authorization; submission requires provider
   authorization.
5. The evaluator is pre-agreed and distinct from the provider. The backend cannot
   forge evaluator approval.
6. Token contract and network are configured once and immutable per deployed
   kernel.
7. ✅ The funding amount must equal the approved quote, which defends against
   budget races.
8. ✅ Every claim or refund follows a rule published before funding.
9. Database outages and worker retries can never duplicate settlement.
10. Signer and policy rotations have a defined, tested effect on existing jobs.

## Test matrix (§10.6)

| Case | Must hold |
|---|---|
| Happy path | Only the provider is paid, after authorized approval |
| Rejection | Client refunded exactly once |
| Expiry | Permitted caller obtains refund after cutoff |
| Premature refund | Fails before deadline / review grace |
| Unauthorized provider | Cannot submit for another provider's job |
| Unauthorized evaluator | Cannot release payout |
| Provider as evaluator | Impossible under the chosen policy |
| Budget race | Mismatched expected budget fails atomically |
| Repeat settlement | Second payout/rejection/refund fails |
| Wrong asset | Funding fails; token is never silently rerouted |
| Malicious hook | Cannot steal escrow or block a rightful expiry refund |
| TTL / archival | Recovery works, or fails explicitly without fund loss |
| Network replay | A testnet quote/signature cannot be replayed on mainnet |
| Malformed proof | Malformed, oversized or untrusted input is rejected |
