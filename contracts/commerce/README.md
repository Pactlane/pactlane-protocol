# commerce

Pactlane's ERC-8183 escrow kernel. A client opens a job, funds it with the exact
agreed budget, the provider submits a work hash, and an independent evaluator
releases payment or refunds the client. Anyone can trigger the refund after
expiry.

One deployment escrows one token. There is no pause or hook. One admin key may
`upgrade` the code in place, keeping the contract ID, jobs and escrow; otherwise
the rules in [`specs/INTERFACES.md`](../../specs/INTERFACES.md) are the only way
funds move.

**Status: in progress** (implementation-plan commits 6–14).
