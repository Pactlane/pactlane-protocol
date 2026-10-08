# evaluation-policy

A hook contract that limits who may call `complete`/`reject` on a job, and under
which deadlines and proof schema.

**Status: not started.** It exists only if the pinned 8183 kernel's hook ABI allows
it without modifying escrow. For v0.1 the evaluator is an authorized account
(ADR-007). Proof-gated settlement is a later, conditional milestone.

See [`specs/EVALUATOR_TRUST.md`](../../specs/EVALUATOR_TRUST.md).
