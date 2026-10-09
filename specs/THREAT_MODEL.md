# Threat model

> **Status: v0.1**, for the kernel in [INTERFACES.md](INTERFACES.md). Written
> before the implementation so tests target threats, not code paths. Each row
> names the test or plan commit that proves the mitigation.

## Assets

- **Escrowed tokens:** the sum of budgets of all Funded and Submitted jobs, held
  by the kernel contract.
- **Job records:** the state, roles, budget and commitments that decide where
  those tokens go.

## Actors

| Actor | Can | Cannot |
|---|---|---|
| Client | create, set provider, propose budget, fund, reject while Open | touch funds after funding; evaluate its own job |
| Provider | propose budget, submit | be paid without the evaluator; evaluate its own work |
| Evaluator | complete or reject a live, funded job | send funds anywhere except the job's provider or client |
| Anyone | claim an expired refund (always to the client), extend TTL, read | anything else |
| Pactlane (deployer) | choose the token at deploy time | anything after deployment: no admin key exists |

**Core property:** tokens leave the kernel only to the funded job's
`provider` (on `complete`) or `client` (on `reject` or `claim_refund`), exactly
once, for exactly the funded budget. No role, key or caller can name another
recipient.

## Trust assumptions

These are not mitigated by the kernel and must be stated to users.

1. **The evaluator is trusted for the verdict.** A dishonest evaluator can pay
   for bad work or refuse good work. The kernel guarantees only that it cannot
   steal or redirect funds. See [EVALUATOR_TRUST.md](EVALUATOR_TRUST.md).
2. **The token issuer is trusted.** USDC's issuer can freeze accounts, including
   the kernel's balance. Funds frozen by the issuer cannot be released by the
   kernel.
3. **Stellar validators are trusted** for ledger time, ordering and availability.
   Network configuration (for example, frozen ledger keys) is outside the kernel's
   control.
4. **Off-chain commitments are only hashes.** The kernel cannot check that
   `spec_hash` or `work_hash` refer to anything meaningful.

## Threats

| # | Threat | Mitigation | Verified by |
|---|---|---|---|
| T1 | Provider approves its own work | `evaluator ≠ provider`, checked whenever a provider is assigned | commits 7, 8, 15 |
| T2 | Client approves its own job and avoids paying | `evaluator ≠ client` | commits 7, 15 |
| T3 | Client pays itself to fake work history | `client ≠ provider` | commits 7, 8, 15 |
| T4 | Budget raised between the client's decision and funding | `fund` requires `expected_budget == budget` | commit 18 |
| T5 | Budget changed after funding | `set_budget` only while Open | commits 8, 16 |
| T6 | Same job paid twice, or paid and refunded | Terminal states are absorbing; state is written before every transfer; Soroban forbids re-entry | commits 16, 18, 19 |
| T7 | Payout and refund race at the deadline | Once `now ≥ expires_at`, only `claim_refund` acts on a funded job | commit 17 |
| T8 | Evaluator never responds, funds stuck | Permissionless `claim_refund` after expiry | commits 13, 17 |
| T9 | Unauthorized caller acts for a role | `require_auth` on the role's stored address, never on an argument | commit 15 |
| T10 | Funding with the wrong token | Token fixed at deploy; clients and manifests check `token()` before funding | commits 6, 18, 28 |
| T11 | Deployed against a non-token address | Constructor calls `token.decimals()` and fails otherwise | commit 6 |
| T12 | Live job's storage archives, making settlement slower and costlier | TTL extended to the network maximum on every write; jobs capped at 90 days, half the maximum TTL. Archival cannot strand funds: the host restores a touched archived entry automatically, at a fee. | commit 14 (remaining TTL on day 90), commit 20 |
| T13 | Archived job entry is treated as missing and recreated | Soroban never presents an archived persistent entry as absent; it must be restored before use. Job IDs come from a monotonic counter and are never reused. | commit 20 |
| T14 | Authorization replayed on another network | Soroban signatures commit to the network passphrase, and kernel IDs differ per network | documented; commit 30 checks the manifest |
| T15 | Arithmetic overflow | `i128` budgets; release profile keeps `overflow-checks = true`; job counter fails with `IdOverflow` | commits 7, B2 |
| T16 | Resource exhaustion through many jobs | Every function is O(1); each job is its own entry paid for by its creator; no iteration over jobs | commit 25 (resource snapshots) |
| T17 | Privileged key upgrades the kernel or drains it | No admin, upgrade or pause function exists | commit 15 (interface has no such function) |
| T18 | Provider cannot receive tokens (no trustline, frozen) | `complete` fails atomically and the job stays Submitted; the client is refunded after expiry. The provider bears this risk. | B4 |
| T19 | Tampered build deployed | Pinned toolchain, `Cargo.lock`, Wasm hash in the manifest, and `verify-deployment.ts` | commits 27–29 |
| T20 | Provider works without being funded | Out of kernel scope. Providers must see `Funded` on chain before starting. | documented |

## Accepted risks for v0.1

- A provider who submits close to `expires_at` may not get reviewed in time and
  goes unpaid. A separate review deadline is a candidate for a later version.
- An Open job never expires. It holds no funds; its client can reject it at any
  time.
