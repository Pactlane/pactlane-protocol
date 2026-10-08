# Interfaces

> **Status: placeholder.** Filled in once the Stellar-8183 kernel revision is pinned
> (backlog #001, #002).

This document will list the exact contract interfaces Pactlane depends on or
exposes: each function's signature, its required authorization, emitted events and
error codes.

## Rule

Every interface is copied from the **pinned upstream ABI** (`stellar contract info
interface`), not from the blueprint. The blueprint's lifecycle is pseudocode:

```text
client.create_job(description_hash, provider?, evaluator, deadline, hook?)
provider.propose_budget(job_id, amount)
client.confirm_and_fund(job_id, expected_budget)
provider.submit(job_id, deliverable_hash)
evaluator.complete(job_id, evaluation_hash) | evaluator.reject(job_id, evaluation_hash)
anyone.claim_refund(job_id)          # after permitted expiry
```

If upstream differs, the wrapper adapts to upstream, not the other way round.

## To document

- [ ] Kernel functions, arguments and `require_auth` subjects
- [ ] Hook callback signatures and call order (before/after)
- [ ] Event topics and payloads, with a version field
- [ ] Error enum
- [ ] How the payment token is configured, and that it is immutable per deployment
