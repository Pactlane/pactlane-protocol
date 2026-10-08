# ERC-8183 mapping

> **Status: placeholder.** Backlog #001 (pin upstream) and #002 (ABI and event
> compatibility report).

How Pactlane's job lifecycle maps onto [ERC-8183](https://ercs.ethereum.org/ERCS/erc-8183)
as implemented by [TrionLabs `stellar-8183`](https://github.com/trionlabs/stellar-8183).

## Pinned upstream

| Item | Value |
|---|---|
| Repository | `trionlabs/stellar-8183` |
| Commit / release | _TBD_ |
| License | _TBD, confirm MIT from the LICENSE file_ |
| Upstream tests reproduced | _TBD_ |
| Audit status | Testnet-only, unaudited (per upstream) |

## Job states

| State | Entered by | Pactlane notes |
|---|---|---|
| Open | client creates job | |
| Funded | client funds exact budget | |
| Submitted | provider submits deliverable hash | |
| Completed | evaluator approves | provider paid in full |
| Rejected | evaluator rejects, or client cancels before funding | client refunded |
| Expired | deadline passed, permitted caller claims | client refunded |

## Divergences from the ERC

_None recorded yet._ Each divergence needs a reason and a test. Features from newer
ERC drafts (e.g. partial settlement) are **not** promised for v0.1 unless the
pinned kernel implements them.
