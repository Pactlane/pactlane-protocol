# Finalized EVM ingestion

This is the ingestion foundation, allocated 30% of the implementation roadmap.
It does not execute mappings, populate entities, or provide the full v0.1 demo.

```bash
cargo build --workspace
superquery-node --ingest-only --project ./project \
  --database-url 'postgres://postgres:postgres@localhost:5432/superquery?schema=erc20' \
  --rpc-url https://your-ethereum-node --start-height 21000000 --end-height 21000999
```

`project.yaml` or `project.yml`, its GraphQL schema, and referenced ABI assets must
be present. The node uses a pinned snapshot of SDK-owned manifest types, documented
in [vendor/sdk](../vendor/sdk/README.md). SDK build artifacts and WASM ABI compatibility
are a later milestone; this command can use a source project directory.

The node verifies the chain ID on every endpoint it uses. HTTP(S) is supported;
WebSocket endpoints are rejected. Providers must support full transaction blocks
and `eth_getLogs` with a `blockHash` filter. Requests have bounded timeouts and
attempt counts; transport/rate errors retry with capped backoff. Permanent RPC
errors fail. Unsupported finalized tags use the configured confirmation depth;
other finality failures stop ingestion. Confirmation depth is a chain-dependent
estimate, not a guarantee against reorgs.

Batches retain at most the configured admission budget. Fetch completion may be
out of order, but headers are delivered and committed in height order. A failed
fetch or commit does not advance the in-memory acknowledgement cursor. Run the
same command after failure to resume from the persisted journal.

The journal stores height, hash, parent hash and timestamp in
`_superquery_ingestion_blocks`. Transactions/logs are fetched and validated but
are not retained. Handler selection counts are logged; no mappings execute.
`_superquery_blocks` and `indexed_height` remain reserved for mapping commits.
Manifest/schema/asset changes fail restart validation instead of reusing a cursor
with different project inputs. Use a new schema for a changed project until
project migrations exist. A changed canonical hash or broken parent link stops
indexing; automatic rewind/replay is pending.

SIGINT/SIGTERM cancels fetching and finishes an active database commit. A second
signal exits immediately. There is no admin/metrics HTTP server yet. Database TLS
and production deployment hardening remain outside this milestone.

## Verification

```bash
cargo fmt --all -- --check
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
DATABASE_URL=postgres://postgres:postgres@localhost:5432/superquery cargo test --workspace
DATABASE_URL=postgres://postgres:postgres@localhost:5432/superquery python3 tests/integration/ingestion_smoke.py
```

The smoke script requires `psql`, uses deterministic local JSON-RPC responses and
creates/removes a unique database schema. It checks finite ingestion, restart,
separate mapping progress, changed-project rejection, wrong-chain rejection,
canonical mismatch refusal and graceful SIGTERM. Unit coverage includes 1,000
out-of-order fake blocks with bounded concurrency, failure acknowledgement,
RPC retries/timeouts, log lineage and SDK Transfer signature selection. An external
network/provider soak run is still pending.

## Remaining scope (70% planning allocation)

- 15%: SDK build artifact and mapping ABI agreement; Wasmtime host functions,
  serialization, limits, and the ERC-20 WASM-to-entity demo.
- 10%: Handler lifecycle, worker scheduling, ordered entity transactions,
  mapping checkpoints and crash recovery.
- 15%: Mutation history, live reorg detection, ancestor search, rewind and replay.
- 5%: Dynamic datasource creation, persistence and deterministic replay.
- 5%: Health/readiness/admin endpoints and runtime metrics.
- 5%: Dictionary integration with RPC parity.
- 10%: Stellar and Solana adapters.
- 5%: Deployment hardening, database TLS, external RPC soak runs, operator
  recovery, compatibility releases and optional Proof of Index design.
