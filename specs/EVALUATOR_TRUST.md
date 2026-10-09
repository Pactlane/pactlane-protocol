# Evaluator trust

> **Status: v0.1.** Who may approve or reject a job, what that approval is
> worth, and how evaluator keys are managed. The contract interface is in
> [INTERFACES.md](INTERFACES.md#evaluation-policy).

## What the kernel guarantees

The kernel fixes each job's `evaluator` address at creation. Only that address
can complete (pay the provider) or reject (refund the client) a funded job, and
only while it is live. The evaluator **cannot** send funds anywhere else, cannot
act after expiry, and cannot be the job's client or provider.

## What it does not guarantee

**The verdict is trusted, not proven.** A dishonest or careless evaluator can pay
for bad work or refuse good work. v0.1 gives *accountability*: every decision
names a signer, and the evaluation evidence hash is recorded on chain. It does
not give trustless AI verification. Docs and UI must say so.

## The evaluation policy

A single account as evaluator has a flaw: the evaluator is fixed per job, so if
that key is lost or compromised, every live job pointing at it is affected and
nothing can be rotated.

So in v0.1, jobs use an **evaluation policy contract** as their evaluator:

| Role | Can | Cannot |
|---|---|---|
| Owner | add and remove signers; hand ownership to a new owner (two-step) | settle a job unless it is also a signer |
| Signer | complete or reject any job whose evaluator is this policy | settle a job in which it is the client or provider |
| Anyone | read signers, owner and the bound kernel | anything else |

- The policy is bound to **one kernel** at deployment and can never be pointed at
  another.
- Any **one** signer can settle. Threshold (M-of-N) approval is a later version.
- The policy **refuses a signer that is the job's client or provider**. The
  kernel only sees the policy's address, so without this check a provider who is
  also a signer could approve its own work.
- Every settlement emits `settled` with the signer, job, verdict and evidence
  hash, so each decision is attributable to one key.

## Rotation and its effect on existing jobs

The signer set is read when a job is settled, not when it is created:

| Change | Effect on jobs already funded |
|---|---|
| Signer removed | It can no longer settle any job, including ones funded before removal |
| Signer added | It can settle any live job pointing at this policy, including older ones |
| Owner transferred | The new owner controls the signer set for every job |

This is what makes the policy useful: a compromised signer is removed once and is
locked out of every job immediately.

## Key compromise

| Compromised key | Worst case | Bounded by |
|---|---|---|
| A signer | Wrongly completes or rejects live submitted jobs: pays a provider for bad work, or refunds a client for good work | Funds only ever reach that job's provider or client. Remove the signer. |
| The owner | Adds its own signer, then as above | Same bound. Owner transfer is two-step, so a typo cannot lose ownership. |
| Both | Same as above | The kernel: no key can redirect escrow to anyone outside the job |

No evaluator key, policy owner or Pactlane key can move escrow to an address that
is not the job's own provider or client.

## Replay

Each settlement is a fresh Soroban authorization with its own nonce, bound to one
network, one contract, one function and its arguments. A verdict cannot be
replayed:

- on another job, because the job ID is part of the signed arguments;
- on the same job, because the kernel refuses a second settlement;
- on another network, because signatures commit to the network passphrase.

## Later milestones

- **M2:** capture 0G Compute TEE proof envelopes, verify them off chain, and
  record the proof hash as the evidence `reason`.
- **M3 (conditional):** require a verified proof on chain before settlement, only
  if the proof format can be verified on Soroban securely and within budget.
- Threshold signatures (M-of-N) for high-value jobs.
