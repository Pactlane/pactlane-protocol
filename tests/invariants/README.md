# invariants

Property tests for the monetary invariants in
[`specs/INVARIANTS.md`](../../specs/INVARIANTS.md). `proptest` drives random
sequences of up to 60 operations across many jobs, including invalid budgets,
unaffordable funding, skewed expected budgets and time jumps past expiry. Every
invariant is checked after every step.

```bash
cargo test -p pactlane-tests --test invariants                   # 64 sequences
PROPTEST_CASES=2000 cargo test -p pactlane-tests --test invariants
```

The test checks balances against what the job states imply, not against a
reference model, so it stays valid as the kernel changes. A differential test
against a separate model is contributor issue B10.
