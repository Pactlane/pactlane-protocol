# invariants

Property tests for the monetary invariants in
[`specs/INVARIANTS.md`](../../specs/INVARIANTS.md). Each test drives random
sequences of job operations and checks that every invariant holds after each step.

**Status: not started.** Waits for the commerce kernel to be pinned. When the first
test lands, add it as a `[[test]]` target in [`tests/Cargo.toml`](../Cargo.toml).
