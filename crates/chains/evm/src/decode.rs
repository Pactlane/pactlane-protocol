use alloy::primitives::{Address, B256};
use serde_json::Value;
use superquery_chain_api::{ChainError, Header, Result};
use crate::rpc::quantity;

pub(crate) fn hash(value: &Value) -> Result<String> {
    value.as_str().and_then(|s| s.parse::<B256>().ok()).map(|h| format!("{h:#x}"))
        .ok_or_else(|| ChainError::Decode("invalid block or transaction hash".into()))
}

pub(crate) fn address(value: &Value) -> Result<String> {
    value.as_str().and_then(|s| s.parse::<Address>().ok()).map(|a| format!("{a:#x}"))
        .ok_or_else(|| ChainError::Decode("invalid EVM address".into()))
}

pub(crate) fn header(value: &Value, expected: u64) -> Result<Header> {
    if value.is_null() { return Err(ChainError::BlockUnavailable { height: expected }); }
    let height = quantity(&value["number"])?;
    if height != expected { return Err(ChainError::Decode("RPC returned the wrong height".into())); }
    let timestamp = i64::try_from(quantity(&value["timestamp"])?).ok()
        .and_then(|t| chrono::DateTime::from_timestamp(t, 0))
        .ok_or_else(|| ChainError::Decode("invalid block timestamp".into()))?;
    Ok(Header { height, hash: hash(&value["hash"])?, parent_hash: Some(hash(&value["parentHash"])?), timestamp: Some(timestamp) })
}
