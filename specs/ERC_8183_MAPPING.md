# ERC-8183 mapping

> **Status: v0.1.** How the Pactlane kernel ([INTERFACES.md](INTERFACES.md)) maps
> onto [ERC-8183 Agentic Commerce](https://ercs.ethereum.org/ERCS/erc-8183) (read
> 2026-10-08).

Pactlane implements its own kernel. TrionLabs
[`stellar-8183`](https://github.com/trionlabs/stellar-8183) (MIT) was studied as
a behavioural reference. **No code was copied.**

## Normative requirements

| ERC requirement | Pactlane |
|---|---|
| Single payment token per contract (SHALL) | ✅ Set in the constructor, immutable |
| Evaluator set at creation (MUST) | ✅ `create_job` argument, never changes |
| Reentrancy protection on transfers (SHALL) | ✅ Soroban forbids contract re-entry; state is written before every transfer |
| `claimRefund` not hookable (SHALL NOT) | ✅ No hooks at all |
| Hooks must not modify escrow state (MUST NOT) | ✅ No hooks at all |
| ERC-2771 trusted forwarder (SHALL, extension) | ➖ Not applicable. Soroban authorization already separates the signer from the transaction submitter. |
| Six states, terminal Completed/Rejected/Expired | ✅ |
| Only the listed transitions are valid | ✅ Plus the expiry rule below, which removes some |

## Functions

| ERC | Pactlane | Difference |
|---|---|---|
| `createJob(provider, evaluator, expiredAt, description, hook)` | `create_job(client, provider?, evaluator, expires_at, spec_hash)` | `client` is explicit (Soroban has no `msg.sender`). Description is a 32-byte hash. No hook. Maximum duration. Role rules. |
| `setProvider(jobId, provider, optParams)` | `set_provider(id, provider)` | No `optParams` |
| `setBudget(jobId, amount, optParams)` | `set_budget(id, actor, amount)` | `actor` is explicit because either party may call it. No `optParams`. |
| `fund(jobId, expectedBudget, optParams)` | `fund(id, expected_budget)` | No `optParams` |
| `submit(jobId, deliverable, optParams)` | `submit(id, work_hash)` | No `optParams` |
| `complete(jobId, reason, optParams)` | `complete(id, reason)` | `reason` is required |
| `reject(jobId, reason, optParams)` | `reject(id, reason?)` | No `optParams` |
| `claimRefund(jobId)` | `claim_refund(id)` | Same; anyone may call |

## Divergences

Each of these is stricter than the ERC, never looser.

| # | Pactlane rule | ERC position | Reason |
|---|---|---|---|
| 1 | No hooks | Optional | Each hook adds revert paths and coupling. Pactlane's evaluator rules are native instead. |
| 2 | `evaluator ≠ client`, `evaluator ≠ provider`, `client ≠ provider` | Evaluator MAY be client; other pairs unspecified | Independent review and no self-payment (blueprint §9.3) |
| 3 | Once `now ≥ expires_at`, `submit`, `complete`, evaluator `reject` and `fund` fail | Unspecified | Payout and refund can never both be possible |
| 4 | `expires_at ≤ now + 90 days` | Future timestamp only | A job cannot outlive its storage TTL |
| 5 | `spec_hash: BytesN<32>` instead of description text | Free-form description | Fixed-size commitment to the TaskSpec; cheaper storage |
| 6 | `complete` requires a `reason` hash | Reason is a field | Binds every payout to evaluation evidence |
| 7 | No platform or evaluator fees | Optional fee in basis points | Smallest economic surface for v0.1 (blueprint ADR-008) |
| 8 | No admin, no upgrade | Unspecified | No privileged key can touch escrow |

## Events

All ten events the ERC says implementations SHOULD emit are emitted with the
same meaning. Names are snake_case: `job_created`, `provider_set`, `budget_set`,
`job_funded`, `job_submitted`, `job_completed`, `job_rejected`, `job_expired`,
`payment_released`, `refunded`.
