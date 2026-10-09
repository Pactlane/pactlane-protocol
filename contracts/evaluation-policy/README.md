# evaluation-policy

A contract that serves as a job's `evaluator`. The kernel fixes each job's
evaluator at creation, so a single account key would be impossible to rotate on a
live job. Pointing jobs at this contract instead lets the owner add and remove
the signer accounts allowed to complete or reject, without touching funded jobs.

**Status: not started.** Specified with
[`specs/EVALUATOR_TRUST.md`](../../specs/EVALUATOR_TRUST.md) in implementation-plan
commit 21.
