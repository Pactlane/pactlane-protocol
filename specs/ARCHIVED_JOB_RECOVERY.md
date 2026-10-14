# Recovering an archived job

This guide covers the existing Pactlane commerce contract and its Stellar CLI
interface. It does not change contract code; the command names and arguments
come from the interface specification in [`specs/INTERFACES.md`](INTERFACES.md).

## What archival means

A live job is kept below the network's maximum lifetime, so it cannot archive
before its expiry. A terminal job can archive later; archival removes the entry
from live state but does not delete the job or its history. See the storage and
TTL rules in [`specs/INTERFACES.md`](INTERFACES.md#storage-and-ttl) and the
archival cases in [`specs/TTL_AND_RECOVERY.md`](TTL_AND_RECOVERY.md#when-something-archives-anyway).

Do not create a replacement job for an archived ID. Job IDs are monotonic and
the host restores an archived persistent entry when a transaction touches it;
the same rules are covered by the archival integration tests in
[`tests/integration/archival.rs`](../tests/integration/archival.rs).

## Prerequisites and variables

Install the Stellar CLI version listed in the repository's
[`README.md`](../README.md#running-tests), configure the target network, and
use a funded source identity. The repository's deployment script demonstrates
the `--id`, `--source-account`, and `--network` form used below in
[`scripts/deploy-testnet.sh`](../scripts/deploy-testnet.sh).

Set these values in the shell before running a command:

```bash
COMMERCE_ID=<deployed-pactlane-commerce-contract-id>
SOURCE_ACCOUNT=<funded-stellar-identity>
NETWORK=testnet
JOB_ID=<numeric-job-id>
```

The contract ID must be the commerce deployment, and `JOB_ID` is the numeric
ID returned when the job was created; the commerce contract and its job ID
sequence are defined in [`specs/INTERFACES.md`](INTERFACES.md#commerce-kernel).

## 1. Inspect the job

Read the job before changing it:

```bash
stellar contract invoke \
  --id "$COMMERCE_ID" \
  --source-account "$SOURCE_ACCOUNT" \
  --network "$NETWORK" \
  -- \
  get_job --id "$JOB_ID"
```

`get_job` is an unauthenticated view and returns the job or `NotFound`, as
specified in [`specs/INTERFACES.md`](INTERFACES.md#views-and-maintenance). The
Stellar CLI generates the function arguments after `--`; its command shape is
documented in the [Stellar CLI manual](https://developers.stellar.org/docs/tools/cli/stellar-cli).

## 2. Renew a job that should remain readable

For an existing job that is not being settled yet, renew its job entry and the
kernel instance:

```bash
stellar contract invoke \
  --id "$COMMERCE_ID" \
  --source-account "$SOURCE_ACCOUNT" \
  --network "$NETWORK" \
  -- \
  extend_ttl --id "$JOB_ID"
```

`extend_ttl` needs no contract signature and extends the job and instance to
the network maximum. It returns `NotFound` only when the job ID does not exist;
these guarantees are defined in [`specs/INTERFACES.md`](INTERFACES.md#views-and-maintenance)
and exercised by [`tests/integration/archival.rs`](../tests/integration/archival.rs).

An archived finished job can be read and renewed through the same call. The
integration test verifies that the job remains unchanged after renewal in
[`tests/integration/archival.rs`](../tests/integration/archival.rs).

## 3. Claim an expired refund

If the inspection shows `Funded` or `Submitted` and the job has passed
`expires_at`, claim its refund:

```bash
stellar contract invoke \
  --id "$COMMERCE_ID" \
  --source-account "$SOURCE_ACCOUNT" \
  --network "$NETWORK" \
  -- \
  claim_refund --id "$JOB_ID"
```

`claim_refund` requires a `Funded` or `Submitted` job and an expiry that has
passed. It needs no contract signature, moves the job to `Expired`, and returns
the budget to the client; the exact preconditions and effects are listed in
[`specs/INTERFACES.md`](INTERFACES.md#claim_refundid).

There is no separate restore command. When this call touches an archived
persistent entry, Soroban restores it and charges the restoration resources as
part of the transaction. The repository's recovery specification describes the
simulation/footprint requirement in [`specs/TTL_AND_RECOVERY.md`](TTL_AND_RECOVERY.md#when-something-archives-anyway),
and the full-refund case is tested in
[`tests/integration/archival.rs`](../tests/integration/archival.rs).

## 4. Verify the result

Read the job again with the inspection command. A successful refund is recorded
as `Expired`; the contract emits `JobExpired` followed by `Refunded`, as defined
in [`specs/INTERFACES.md`](INTERFACES.md#claim_refundid) and verified by the
refund tests in [`tests/unit/commerce/claim_refund.rs`](../tests/unit/commerce/claim_refund.rs).

If the job is still `Funded` or `Submitted`, check the reported expiry before
retrying. If the CLI reports `NotExpired`, the contract is correctly refusing a
refund before its deadline, as covered by
[`tests/unit/commerce/claim_refund.rs`](../tests/unit/commerce/claim_refund.rs).

## Limitations

This procedure covers Pactlane's job entry and kernel instance. Token entries
have their own TTL policy, and missed events must be recovered through an
archive or history service; those boundaries are documented in
[`specs/TTL_AND_RECOVERY.md`](TTL_AND_RECOVERY.md#not-covered-by-the-kernel).
