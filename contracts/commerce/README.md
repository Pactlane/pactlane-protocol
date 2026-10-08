# commerce

The ERC-8183-style escrow kernel: create, fund, submit, complete/reject, refund.

**Status: empty, on purpose.** The plan is to use a pinned upstream
[TrionLabs `stellar-8183`](https://github.com/trionlabs/stellar-8183) deployment.
Source lands here only if a required behaviour cannot be reached through the
kernel's hooks. In that case this directory holds an audited derivative, with every
divergence recorded in [`specs/ERC_8183_MAPPING.md`](../../specs/ERC_8183_MAPPING.md).

Gate: blueprint Phase 0. Reproduce upstream's tests and pin a revision before
deciding.
