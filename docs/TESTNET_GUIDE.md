# Testing on Stellar testnet

Contract IDs, Wasm hashes and constructor arguments live in
[`deployments/testnet.json`](../deployments/testnet.json). Read them from
there; they stay the same across upgrades.

## 1. Get a funded testnet account

Every wallet that signs (client, provider, evaluator) needs XLM for fees.

```bash
stellar keys generate my-client --network testnet --fund
```

Or use Friendbot directly: `https://friendbot.stellar.org/?addr=<G... address>`.
Browser wallets such as Freighter have a "Fund with Friendbot" button on testnet.

## 2. Add a USDC trustline

The kernel escrows Circle's testnet USDC, issued by
`GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5`. A classic account
can hold it only after adding a trustline:

```bash
stellar tx new change-trust --source my-client --network testnet \
  --line USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5
```

In Freighter: Manage assets, Add an asset, search USDC, and pick the Circle
issuer above. Providers need the trustline too, or `complete` fails (threat
T18) and the client is refunded after expiry instead.

## 3. Get testnet USDC

Request it from <https://faucet.circle.com>: choose **Stellar** and
**Testnet**, and paste the client's address. Amounts on chain use 7
decimals, so 1 USDC is `10000000` base units.

## 4. Check the balance

```bash
stellar contract invoke --network testnet --source my-client \
  --id CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA \
  --send=no -- balance --id <G... address>
```

## 5. Run a job

```bash
K=CB3DJM7B7THZYCYINV22NATMRV3AFG4XX7235KYP2VVGOTRG2CGVKIQS
stellar contract invoke --id $K --network testnet --source my-client -- \
  create_job --client <client> --provider <provider> --evaluator <evaluator> \
  --expires_at <unix seconds, at most 90 days ahead> --spec_hash <64 hex chars>
stellar contract invoke --id $K --network testnet --source my-client -- \
  set_budget --id 1 --actor <client> --amount 10000000
stellar contract invoke --id $K --network testnet --source my-client -- \
  fund --id 1 --expected_budget 10000000
```

Then the provider calls `submit`, and the evaluator calls `complete` or
`reject`. After `expires_at`, anyone can call `claim_refund`.
