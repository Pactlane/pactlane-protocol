# commerce

Pactlane's ERC-8183 escrow kernel. A client opens a job, funds it with the exact
agreed budget, the provider submits a work hash, and an independent evaluator
releases payment or refunds the client. Anyone can trigger the refund after
expiry.

One deployment escrows one token. There is no admin, upgrade, pause or hook: the
rules in [`specs/INTERFACES.md`](../../specs/INTERFACES.md) are the only way funds
move.

**Status: specified, not yet implemented** (implementation-plan commits 6–14).
