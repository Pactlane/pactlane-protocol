# Evaluator trust

> **Status: placeholder.** Backlog #007 (evaluator trust and authorization RFC).

Who may approve or reject a job, and what that approval is worth.

## v0.1 model (ADR-007)

A **contract-authorized evaluator account** calls `complete` or `reject` with an
evidence hash. This gives evaluator *accountability*. It is **not** trustless AI
verification, and the docs and UI must say so.

## Requirements

- The evaluator cannot be the provider. The client can act as evaluator only if
  the selected policy explicitly allows it.
- The evaluation binds `network + commerce_contract + job_id + task_hash +
  result_hash + verdict + deadline + evaluator_identity`.
- Duplicate or replayed verdicts are rejected, and so are signers that have been
  removed.
- The evaluator judges the exact committed artifact, never an unpinned URL.
- Deterministic checks run before any subjective LLM score.

## Later milestones

- **M2:** capture 0G Compute TEE proof envelopes; verify them off-chain; record
  the proof hash on chain.
- **M3 (conditional):** on-chain proof gating, only if the proof format can be
  verified on Soroban securely and within budget.

## To decide

- [ ] Who controls evaluator keys in v0.1, and how they are rotated and revoked
- [ ] How rotation affects jobs that are already funded
