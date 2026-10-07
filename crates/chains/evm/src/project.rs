//! Compile SDK filters once, before processing blocks.
use crate::{EvmBlock, EvmLogFilter, EvmTransactionFilter, TopicFilter};
use alloy::{
    json_abi::{Event, Function},
    primitives::{Address, B256},
};
use superquery_chain_api::{ChainError, Result};
use superquery_config::{manifest::HandlerFilter, ProjectManifest};

enum InputFilter {
    Block(Option<u64>),
    Log(EvmLogFilter),
    Transaction(EvmTransactionFilter),
}

/// Validated handler selection for one datasource.
pub struct PreparedHandler {
    /// Export named by the SDK manifest.
    pub function: String,
    start: u64,
    end: Option<u64>,
    filter: InputFilter,
}

impl PreparedHandler {
    /// Number of inputs selected; this does not execute any mappings.
    pub fn matching_inputs(&self, block: &EvmBlock) -> usize {
        if block.number < self.start || self.end.is_some_and(|end| block.number > end) {
            return 0;
        }
        match &self.filter {
            InputFilter::Block(modulo) => {
                usize::from(modulo.is_none_or(|m| block.number.is_multiple_of(m)))
            }
            InputFilter::Log(filter) => block
                .logs
                .iter()
                .filter(|l| filter.matches(&l.view()))
                .count(),
            InputFilter::Transaction(filter) => block
                .transactions
                .iter()
                .filter(|t| filter.matches(&t.view()))
                .count(),
        }
    }
}

fn address(raw: Option<&str>) -> Result<Option<String>> {
    raw.map(|s| {
        s.parse::<Address>()
            .map(|a| format!("{a:#x}"))
            .map_err(|_| ChainError::Decode("invalid manifest address".into()))
    })
    .transpose()
}

/// Reject unsupported filters and normalize ABI signatures to hashes/selectors.
pub fn prepare_handlers(project: &ProjectManifest) -> Result<Vec<PreparedHandler>> {
    let mut prepared = Vec::new();
    for ds in &project.data_sources {
        for key in ds.options.keys() {
            if !matches!(key, "address" | "abi") {
                return Err(ChainError::Decode(format!(
                    "unsupported datasource option {key}"
                )));
            }
        }
        if ds.options.contains("address") && ds.options.get_str("address").is_none() {
            return Err(ChainError::Decode(
                "datasource address must be a string".into(),
            ));
        }
        let contract = address(ds.options.get_str("address"))?;
        for handler in &ds.handlers {
            let bad = || {
                ChainError::Decode(format!(
                    "invalid or unsupported filter for {}",
                    handler.handler
                ))
            };
            let filter = match (handler.kind.as_str(), &handler.filter) {
                ("evm/LogHandler", filter) => {
                    let mut topics = Vec::new();
                    if let Some(HandlerFilter::Log(log)) = filter {
                        let event = Event::parse(&log.event).map_err(|_| bad())?;
                        topics.push(TopicFilter::One(format!("{:#x}", event.selector())));
                        for topic in &log.topics {
                            let topic = topic.parse::<B256>().map_err(|_| bad())?;
                            topics.push(TopicFilter::One(format!("{topic:#x}")));
                        }
                        if topics.len() > 4 {
                            return Err(bad());
                        }
                    } else if filter.is_some() {
                        return Err(bad());
                    }
                    InputFilter::Log(EvmLogFilter {
                        address: contract.clone(),
                        topics,
                    })
                }
                ("evm/TransactionHandler", filter) => {
                    let mut tx = EvmTransactionFilter {
                        to: contract.clone(),
                        ..Default::default()
                    };
                    if let Some(HandlerFilter::Transaction(f)) = filter {
                        tx.function = Some(format!(
                            "{:#x}",
                            Function::parse(&f.function).map_err(|_| bad())?.selector()
                        ));
                        tx.from = address(f.from.as_deref())?;
                        if let Some(to) = address(f.to.as_deref())? {
                            if contract.as_ref().is_some_and(|c| c != &to) {
                                return Err(bad());
                            }
                            tx.to = Some(to);
                        }
                    } else if filter.is_some() {
                        return Err(bad());
                    }
                    InputFilter::Transaction(tx)
                }
                ("evm/BlockHandler", None) => InputFilter::Block(None),
                ("evm/BlockHandler", Some(HandlerFilter::Block(f)))
                    if f.modulo != Some(0) && f.timestamp.is_none() =>
                {
                    InputFilter::Block(f.modulo)
                }
                _ => return Err(bad()),
            };
            prepared.push(PreparedHandler {
                function: handler.handler.clone(),
                start: project.effective_start_block(ds),
                end: ds.end_block,
                filter,
            });
        }
    }
    Ok(prepared)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sdk_transfer_signature_compiles_and_selects_only_matching_logs() {
        let project =
            superquery_config::LoadedProject::load("../../../tests/fixtures/sdk-erc20").unwrap();
        let handlers = prepare_handlers(&project.manifest).unwrap();
        let mut block = EvmBlock {
            number: 21_000_000,
            hash: String::new(),
            parent_hash: String::new(),
            timestamp: 0,
            transactions: vec![],
            logs: vec![crate::EvmLog {
                address: "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".into(),
                topics: vec![
                    "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef".into(),
                ],
                data: "0x".into(),
                log_index: 0,
                transaction_hash: String::new(),
                transaction_index: 0,
            }],
        };
        assert_eq!(handlers[0].matching_inputs(&block), 1);
        block.logs[0].address = "0x0000000000000000000000000000000000000000".into();
        assert_eq!(handlers[0].matching_inputs(&block), 0);
    }
}
