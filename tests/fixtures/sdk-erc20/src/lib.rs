//! ERC-20 Transfer indexer.
//!
//! Run `superquery codegen` before building: `Transfer` (the decoded event)
//! and `TransferEntity` (the stored row) are both generated from
//! `abis/ERC20.json` and `schema.graphql`.

use superquery_sdk::prelude::*;

mod generated;

use generated::contracts::erc20::Transfer;
use generated::entities::Transfer as TransferEntity;

/// Store every ERC-20 transfer as a row.
#[handler]
pub async fn handle_transfer(event: EvmLog<Transfer>) -> Result<()> {
    TransferEntity {
        // `<tx hash>-<log index>` is unique and stable across re-indexing.
        id: event.id(),
        from: event.params.from.to_string(),
        to: event.params.to.to_string(),
        value: BigInt::new(event.params.value.to_string()),
        block_number: BigInt::from(event.block.number),
        timestamp: Timestamp::from_secs(event.block.timestamp as i64),
        transaction_hash: Bytes::new(event.transaction_hash.to_vec()),
    }
    .save()
    .await
}
