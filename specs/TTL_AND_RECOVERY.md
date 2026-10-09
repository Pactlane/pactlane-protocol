# TTL and recovery

> **Status: v0.1.** Behaviour verified by `tests/unit/commerce/ttl.rs` and
> `tests/integration/archival.rs`. Step-by-step CLI restoration is contributor
> issue B21.

Soroban charges rent for ledger entries. When an entry's TTL (time to live, in
ledgers) runs out, the entry is **archived**: it leaves live state but is not
deleted. This document says how the kernel manages TTL, and what happens when
something archives anyway.

## Network limits

Read from testnet with `stellar network settings` on 2026-10-08:

| Setting | Ledgers | ≈ Time at 5 s/ledger |
|---|---:|---:|
| `max_entry_ttl` | 3,110,400 | 180 days |
| `min_persistent_ttl` | 120,960 | 7 days |
| `min_temporary_ttl` | 720 | 1 hour |

## What the kernel stores

| Data | Storage | TTL policy |
|---|---|---|
| Payment token, job counter | instance | extended to the maximum on every write |
| Contract code (Wasm) | code entry | extended with the instance |
| `Job(id)` | persistent | extended to the maximum on every write, and by `extend_ttl(id)` |

The kernel uses no temporary storage, which would be deleted rather than archived.

## Why live jobs never archive

Every state change writes the job, and every write extends both the job and the
instance to the network maximum (about 180 days). Jobs last at most 90 days
(`MAX_JOB_DURATION`). So when a job expires, at least half the maximum TTL is
left since its last write. `ttl.rs` checks the exact remaining TTL on day 90 of a
maximum-length job.

Anyone can call `extend_ttl(id)` without a signature to renew a job and the
kernel instance again, for example an indexer keeping old jobs readable.

## When something archives anyway

A **funded job whose refund nobody claims** for about 180 days after its last
write archives. So do completed and rejected jobs, and eventually the kernel's
instance and code if the kernel sees no activity at all.

**Archival never strands funds.** When a transaction touches an archived
persistent entry, the host restores it automatically and charges for it. The
restored entries are counted as disk reads and rent is charged to bring them
back (soroban-env-host 29, `handle_maybe_expired_entry`). On the network this is
[CAP-66](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0066.md)
(protocol 23). The transaction lists archived keys in `archivedSorobanEntries`, and
Stellar RPC's simulation fills that in. A client that simulates before sending
needs no extra step. One that builds footprints by hand without declaring archived
keys fails with `INVOKE_HOST_FUNCTION_ENTRY_ARCHIVED`.

`archival.rs` checks this against the compiled Wasm. A refund claimed 200 days
after funding, with the job, the kernel instance and the code all archived, still
returns the full budget to the client. It reads at least 3 more entries from
disk than a refund claimed on time.

## Recovery checklist

| Situation | Effect | What to do |
|---|---|---|
| Live job | None. TTL covers its whole life. | Nothing |
| Expired, funded, unclaimed for ~180 days | Refund costs restoration rent | Call `claim_refund` as usual; simulation adds the restore |
| Finished job archived | `get_job` needs restoration | Read it (restoration is automatic), or call `extend_ttl` to keep it live |
| Kernel idle ~180 days | First call restores instance and code | Any call restores; `extend_ttl` on any job renews the instance |

## Not covered by the kernel

- The **token's** entries (balances, its instance) have their own TTLs, managed
  by the token contract and network. Tests show a refund may also restore token
  entries.
- **Events** are not ledger entries. Indexers that missed events must use
  archive or history services; job state remains readable from the kernel.
