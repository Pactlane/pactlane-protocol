# Interfaces

> **Status: v0.1 specification.** The source of truth for the `commerce` kernel.
> Code follows this document; a change to behaviour changes this document in the
> same commit.

## Commerce kernel

One kernel deployment escrows **one** SEP-41 token (testnet USDC) for any number
of jobs. It has **no admin, no upgrade path, no pause, and no hooks**. Once
deployed, nobody (including Pactlane) can move escrowed funds except through the
rules below. Fixes ship as a new deployment with a new contract ID.

### Types

```rust
enum JobState { Open, Funded, Submitted, Completed, Rejected, Expired }

struct Job {
    id: u64,
    client: Address,             // creates, funds, may reject while Open
    provider: Option<Address>,   // set once; does the work
    evaluator: Address,          // fixed at creation; completes or rejects
    spec_hash: BytesN<32>,       // commitment to the off-chain TaskSpec
    budget: i128,                // token base units; 0 until set
    expires_at: u64,             // ledger timestamp, seconds
    state: JobState,
    work_hash: Option<BytesN<32>>,  // set by submit
    reason: Option<BytesN<32>>,     // set by complete / reject
}
```

`Completed`, `Rejected` and `Expired` are **terminal**: no function changes a
terminal job.

### Constants

| Name | Value | Why |
|---|---|---|
| `MAX_JOB_DURATION` | 7,776,000 s (90 days) | Half the network's maximum entry TTL (testnet `max_entry_ttl` = 3,110,400 ledgers ≈ 180 days, read 2026-10-08), so a job never outlives its storage |

### Role rules

Checked everywhere a role is assigned (`create_job`, `set_provider`):

- `evaluator ≠ client`. The buyer cannot approve their own job.
- `evaluator ≠ provider`. The worker cannot approve their own work.
- `client ≠ provider`. A client cannot pay itself.

Violations fail with `RoleConflict`.

### Expiry rule

Let `now` be the ledger timestamp. While `now < expires_at` the job is **live**.
Once `now ≥ expires_at`, the only action that changes a Funded or Submitted job is
`claim_refund`. There is no window in which both payout and refund are possible.

An Open job holds no funds and never expires on its own. Its client may reject it
at any time.

### Functions

Authorization is Soroban `require_auth` on the named address. "Fails" means the
call returns the error and changes nothing.

Checks run in the order listed, after the job is loaded (`NotFound` first), and
the first failing check decides the error. Tests depend on this order.

#### `__constructor(token: Address)`

Binds the payment token forever. Calls `token.decimals()` once, so deploying
against an address that is not a token contract fails.

#### `create_job(client, provider: Option<Address>, evaluator, expires_at: u64, spec_hash: BytesN<32>) -> u64`

| | |
|---|---|
| Auth | `client` |
| Requires | `now < expires_at ≤ now + MAX_JOB_DURATION`, otherwise `BadExpiry` · role rules, otherwise `RoleConflict` |
| Effect | New job in `Open` with `budget = 0`. IDs start at 1 and increase by 1. |
| Event | `JobCreated` |

#### `set_provider(id, provider: Address)`

| | |
|---|---|
| Auth | job's `client` |
| Requires | `Open`, otherwise `BadState` · live, otherwise `Expired` · provider not yet set, otherwise `ProviderAlreadySet` · role rules |
| Event | `ProviderSet` |

#### `set_budget(id, actor: Address, amount: i128)`

| | |
|---|---|
| Auth | `actor` |
| Requires | `actor` is the job's client or provider, otherwise `BadActor` · `Open` · live · `amount > 0`, otherwise `BadBudget` |
| Effect | Replaces the budget. Either party may propose; `fund` checks that the client agrees. |
| Event | `BudgetSet` |

#### `fund(id, expected_budget: i128)`

| | |
|---|---|
| Auth | job's `client` (also authorizes the token transfer) |
| Requires | `Open` · live · provider set, otherwise `NoProvider` · `budget > 0`, otherwise `BadBudget` · `budget == expected_budget`, otherwise `BudgetMismatch` |
| Effect | State → `Funded`, **then** transfers `budget` client → kernel. |
| Event | `JobFunded` |

`expected_budget` defends against a budget changed between the client deciding
and the transaction landing.

#### `submit(id, work_hash: BytesN<32>)`

| | |
|---|---|
| Auth | job's `provider` |
| Requires | `Funded` · live |
| Effect | Stores `work_hash`. State → `Submitted`. |
| Event | `JobSubmitted` |

#### `complete(id, reason: BytesN<32>)`

| | |
|---|---|
| Auth | job's `evaluator` |
| Requires | `Submitted` · live |
| Effect | Stores `reason` (evaluation evidence hash, required). State → `Completed`, **then** transfers `budget` kernel → provider. |
| Events | `JobCompleted`, `PaymentReleased` |

#### `reject(id, reason: Option<BytesN<32>>)`

| State | Auth | Requires | Effect | Events |
|---|---|---|---|---|
| `Open` | job's `client` | none (expiry ignored) | State → `Rejected`. No transfer. | `JobRejected` |
| `Funded` / `Submitted` | job's `evaluator` | live | State → `Rejected`, **then** transfers `budget` kernel → client | `JobRejected`, `Refunded` |
| terminal | | fails `BadState` | | |

#### `claim_refund(id)`

| | |
|---|---|
| Auth | **none**: anyone may call it, so a refund never depends on one party being online |
| Requires | `Funded` or `Submitted`, otherwise `BadState` · `now ≥ expires_at`, otherwise `NotExpired` |
| Effect | State → `Expired`, **then** transfers `budget` kernel → client. |
| Events | `JobExpired`, `Refunded` |

#### Views and maintenance

| Function | Auth | Returns / effect |
|---|---|---|
| `get_job(id) -> Job` | none | the job, or `NotFound` |
| `job_count() -> u64` | none | number of jobs ever created |
| `token() -> Address` | none | the payment token |
| `extend_ttl(id)` | none | extends the job's and the instance's TTL to the network maximum; `NotFound` if absent |

### Storage and TTL

| Data | Storage | TTL |
|---|---|---|
| token, job counter | instance | extended to max on every write |
| `Job(id)` | persistent | extended to max on every write and by `extend_ttl` |

With the network maximum at about 180 days and jobs capped at 90, a live job's
entry cannot archive between its last write and its expiry. Terminal jobs may
archive later. Their history remains in events.

Archival never strands funds. When a transaction touches an archived persistent
entry, the host restores it automatically and charges a restore fee
(soroban-env-host 29, `handle_maybe_expired_entry`). TTL management keeps live
jobs off that path, so settling one never costs a restore and indexers can always
read it.

### Errors

Codes are part of the ABI and never renumbered.

| Code | Name | Meaning |
|---:|---|---|
| 1 | `NotFound` | no job with that ID |
| 2 | `BadState` | action not allowed in the job's current state |
| 3 | `BadActor` | `set_budget` caller is neither client nor provider |
| 4 | `BadExpiry` | `expires_at` not in the future, or beyond `MAX_JOB_DURATION` |
| 5 | `Expired` | action needs a live job but `now ≥ expires_at` |
| 6 | `NotExpired` | `claim_refund` before `expires_at` |
| 7 | `BadBudget` | budget not positive |
| 8 | `BudgetMismatch` | `expected_budget` differs from the stored budget |
| 9 | `NoProvider` | funding a job with no provider |
| 10 | `ProviderAlreadySet` | provider already assigned |
| 11 | `RoleConflict` | two roles share one address |
| 12 | `IdOverflow` | job counter exhausted |

Failed `require_auth` is a host error, not one of these codes.

### Events

Each event's first topic is its name. Every event carries the job `id` as a topic.
Data is a map keyed by field name. Events with an optional field (`job_created`,
`job_rejected`) always include every key, with void for `None`. Payload fields are
append-only; removing or retyping one requires a new kernel version.

The Rust definitions live in `contracts/interfaces`, which also holds the
`Commerce` trait. The kernel implements that trait, so its ABI cannot drift from
this document's function list without a compile error.

| Event | Topics | Data |
|---|---|---|
| `job_created` | id, client | provider, evaluator, expires_at, spec_hash |
| `provider_set` | id, provider | none |
| `budget_set` | id, actor | amount |
| `job_funded` | id, client | amount |
| `job_submitted` | id, provider | work_hash |
| `job_completed` | id, evaluator | reason |
| `job_rejected` | id, rejector | reason |
| `job_expired` | id | none |
| `payment_released` | id, provider | amount |
| `refunded` | id, client | amount |

## Evaluation policy

Specified in commit 21 of the implementation plan, together with
[EVALUATOR_TRUST.md](EVALUATOR_TRUST.md).
