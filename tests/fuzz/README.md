# fuzz

Coverage-guided fuzzing of contract entry points: malformed proofs, oversized
inputs, and hostile hook responses.

**Status: not started.** `cargo-fuzz` needs nightly and its own crate, so this
becomes a separate crate outside the main workspace when the first target is
written. See the Stellar fuzzing guide:
<https://developers.stellar.org/docs/build/guides/testing/fuzzing>.
