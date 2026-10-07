//! Handler filters.
//!
//! Filters are the difference between a project that indexes one contract's
//! `Transfer` events and one that decodes every log on the chain, so they are
//! typed rather than free-form. A filter that does not match its handler kind
//! is a validation error, not a runtime surprise.

use serde::{Deserialize, Serialize};

/// A filter attached to a handler.
///
/// Untagged: the manifest spells filters by their fields (`event:`,
/// `function:`, `modulo:`) rather than by an explicit discriminator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HandlerFilter {
    /// Matches logs/events, e.g. `event: "Transfer(address,address,uint256)"`.
    Log(LogFilter),
    /// Matches transactions/calls by signature.
    Transaction(TransactionFilter),
    /// Matches blocks, optionally every Nth block.
    Block(BlockFilter),
}

/// Filter for log/event handlers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LogFilter {
    /// The event signature, e.g. `Transfer(address,address,uint256)`.
    pub event: String,

    /// Optional indexed-topic constraints, positionally after topic0.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<String>,
}

/// Filter for transaction/call handlers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransactionFilter {
    /// The function signature, e.g. `transfer(address,uint256)`.
    pub function: String,

    /// Optional `from` address constraint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,

    /// Optional `to` address constraint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
}

/// Filter for block handlers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlockFilter {
    /// Run only on heights divisible by this value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modulo: Option<u64>,

    /// Run only on a timestamp cron schedule, for families that support it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

impl HandlerFilter {
    /// A short label naming which filter shape this is, for diagnostics.
    pub const fn shape(&self) -> &'static str {
        match self {
            HandlerFilter::Log(_) => "log filter (`event`)",
            HandlerFilter::Transaction(_) => "transaction filter (`function`)",
            HandlerFilter::Block(_) => "block filter (`modulo`/`timestamp`)",
        }
    }
}
