use crate::rpc::quantity;
use alloy::primitives::{Address, B256};
use serde_json::Value;
use superquery_chain_api::{ChainError, Header, Result};

pub(crate) fn hash(value: &Value) -> Result<String> {
    value
        .as_str()
        .and_then(|s| s.parse::<B256>().ok())
        .map(|h| format!("{h:#x}"))
        .ok_or_else(|| ChainError::Decode("invalid block or transaction hash".into()))
}

pub(crate) fn address(value: &Value) -> Result<String> {
    value
        .as_str()
        .and_then(|s| s.parse::<Address>().ok())
        .map(|a| format!("{a:#x}"))
        .ok_or_else(|| ChainError::Decode("invalid EVM address".into()))
}

pub(crate) fn header(value: &Value, expected: u64) -> Result<Header> {
    if value.is_null() {
        return Err(ChainError::BlockUnavailable { height: expected });
    }
    let height = quantity(&value["number"])?;
    if height != expected {
        return Err(ChainError::Decode("RPC returned the wrong height".into()));
    }
    let timestamp = i64::try_from(quantity(&value["timestamp"])?)
        .ok()
        .and_then(|t| chrono::DateTime::from_timestamp(t, 0))
        .ok_or_else(|| ChainError::Decode("invalid block timestamp".into()))?;
    Ok(Header {
        height,
        hash: hash(&value["hash"])?,
        parent_hash: Some(hash(&value["parentHash"])?),
        timestamp: Some(timestamp),
    })
}

fn bytes(value: &Value) -> Result<String> {
    value
        .as_str()
        .and_then(|s| s.parse::<alloy::primitives::Bytes>().ok())
        .map(|b| format!("{b:#x}"))
        .ok_or_else(|| ChainError::Decode("invalid RPC byte string".into()))
}

pub(crate) fn block(value: &Value, logs: &Value, expected: u64) -> Result<crate::EvmBlock> {
    let header = header(value, expected)?;
    let mismatch = || ChainError::Decode("inconsistent block, transaction or log identity".into());
    let mut transactions = Vec::new();
    let mut hashes = std::collections::HashSet::new();
    for (index, tx) in value["transactions"]
        .as_array()
        .ok_or_else(mismatch)?
        .iter()
        .enumerate()
    {
        let tx_hash = hash(&tx["hash"])?;
        if quantity(&tx["transactionIndex"])? != index as u64
            || quantity(&tx["blockNumber"])? != expected
            || hash(&tx["blockHash"])? != header.hash
            || !hashes.insert(tx_hash.clone())
        {
            return Err(mismatch());
        }
        let raw_value = tx["value"].as_str().ok_or_else(mismatch)?;
        let value = raw_value
            .parse::<alloy::primitives::U256>()
            .map_err(|_| mismatch())?;
        transactions.push(crate::EvmTransaction {
            hash: tx_hash,
            transaction_index: index as u64,
            from: address(&tx["from"])?,
            to: if tx["to"].is_null() {
                None
            } else {
                Some(address(&tx["to"])?)
            },
            value: value.to_string(),
            input: bytes(&tx["input"])?,
        });
    }
    let mut decoded_logs = Vec::new();
    for log in logs.as_array().ok_or_else(mismatch)? {
        let tx_index = quantity(&log["transactionIndex"])?;
        let tx = usize::try_from(tx_index)
            .ok()
            .and_then(|i| transactions.get(i))
            .ok_or_else(mismatch)?;
        if log["removed"].as_bool() != Some(false)
            || hash(&log["blockHash"])? != header.hash
            || quantity(&log["blockNumber"])? != expected
            || hash(&log["transactionHash"])? != tx.hash
        {
            return Err(mismatch());
        }
        let topics = log["topics"]
            .as_array()
            .ok_or_else(mismatch)?
            .iter()
            .map(hash)
            .collect::<Result<Vec<_>>>()?;
        if topics.len() > 4 {
            return Err(mismatch());
        }
        decoded_logs.push(crate::EvmLog {
            address: address(&log["address"])?,
            topics,
            data: bytes(&log["data"])?,
            log_index: quantity(&log["logIndex"])?,
            transaction_hash: tx.hash.clone(),
            transaction_index: tx_index,
        });
    }
    decoded_logs.sort_by_key(|log| log.log_index);
    for (index, log) in decoded_logs.iter().enumerate() {
        if log.log_index != index as u64 {
            return Err(mismatch());
        }
    }
    Ok(crate::EvmBlock {
        number: expected,
        hash: header.hash,
        parent_hash: header.parent_hash.ok_or_else(mismatch)?,
        timestamp: quantity(&value["timestamp"])?,
        transactions,
        logs: decoded_logs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    pub(super) fn fixture() -> (Value, Value) {
        let hash = format!("0x{}", "ab".repeat(32));
        let tx_hash = format!("0x{}", "cd".repeat(32));
        let address = format!("0x{}", "12".repeat(20));
        (
            json!({"number":"0x2", "hash":hash, "parentHash":format!("0x{}", "00".repeat(32)), "timestamp":"0x1234",
            "transactions":[{"hash":tx_hash, "blockHash":hash, "blockNumber":"0x2", "transactionIndex":"0x0", "from":address, "to":null, "value":"0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff", "input":"0x"}]}),
            json!([{"blockHash":hash, "blockNumber":"0x2", "transactionHash":tx_hash, "transactionIndex":"0x0", "logIndex":"0x0", "removed":false,
            "address":address, "topics":[], "data":"0x"}]),
        )
    }

    #[test]
    fn decodes_full_width_value_and_contract_creation() {
        let (value, logs) = fixture();
        let b = block(&value, &logs, 2).unwrap();
        assert_eq!(
            b.transactions[0].value,
            alloy::primitives::U256::MAX.to_string()
        );
        assert_eq!(b.transactions[0].to, None);
        assert_eq!(b.logs[0].transaction_hash, b.transactions[0].hash);
    }

    #[test]
    fn rejects_mixed_forks_removed_logs_and_missing_transactions() {
        let (value, mut logs) = fixture();
        logs[0]["blockHash"] = json!(format!("0x{}", "ff".repeat(32)));
        assert!(block(&value, &logs, 2).is_err());
        let (value, mut logs) = fixture();
        logs[0]["removed"] = json!(true);
        assert!(block(&value, &logs, 2).is_err());
        let (value, mut logs) = fixture();
        logs[0]["transactionIndex"] = json!("0x1");
        assert!(block(&value, &logs, 2).is_err());
        assert!(header(&value, 3).is_err());
        assert!(header(&Value::Null, 2).is_err());
    }
}
