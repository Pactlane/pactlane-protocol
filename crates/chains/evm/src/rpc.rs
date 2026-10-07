use crate::adapter::EvmAdapterConfig;
use alloy::providers::{Provider, RootProvider};
use serde_json::{json, Value};
use std::time::Duration;
use superquery_chain_api::{ChainError, Result};

pub(crate) struct RpcClient {
    providers: Vec<RootProvider>,
    chain_id: u64,
    attempts: u32,
    timeout: Duration,
}

impl RpcClient {
    pub(crate) fn new(config: &EvmAdapterConfig) -> Result<Self> {
        if config.endpoints.is_empty() || config.max_retries == 0 || config.timeout_secs == 0 {
            return Err(ChainError::Other(
                "RPC requires endpoints, positive attempts and timeout".into(),
            ));
        }
        let providers = config
            .endpoints
            .iter()
            .map(|raw| {
                let url = superquery_config::endpoint::validate_endpoint(raw)
                    .map_err(ChainError::Other)?;
                Ok(RootProvider::new_http(url))
            })
            .collect::<Result<Vec<_>>>()?;
        let chain_id = config
            .chain_id
            .parse()
            .map_err(|_| ChainError::Other("invalid decimal chain id".into()))?;
        Ok(Self {
            providers,
            chain_id,
            attempts: config.max_retries,
            timeout: Duration::from_secs(config.timeout_secs),
        })
    }

    pub(crate) async fn request(&self, method: &'static str, params: Value) -> Result<Value> {
        for attempt in 0..self.attempts {
            let provider = &self.providers[attempt as usize % self.providers.len()];
            let result = tokio::time::timeout(self.timeout, async {
                let id = self.call(provider, "eth_chainId", json!([])).await?;
                let actual = quantity(&id)?;
                if actual != self.chain_id {
                    return Err(ChainError::ChainIdMismatch {
                        expected: self.chain_id.to_string(),
                        actual: actual.to_string(),
                    });
                }
                if method == "eth_chainId" {
                    Ok(id)
                } else {
                    self.call(provider, method, params.clone()).await
                }
            })
            .await
            .unwrap_or_else(|_| Err(ChainError::Transport("RPC request timed out".into())));
            match result {
                Ok(value) => return Ok(value),
                Err(error) if error.is_retryable() && attempt + 1 < self.attempts => {
                    let delay = 100u64.saturating_mul(1u64 << attempt.min(6)).min(5_000);
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
                Err(error) => return Err(error),
            }
        }
        Err(ChainError::Other("RPC attempt budget exhausted".into()))
    }

    async fn call(
        &self,
        provider: &RootProvider,
        method: &'static str,
        params: Value,
    ) -> Result<Value> {
        provider
            .raw_request(method.into(), params)
            .await
            .map_err(|error| {
                if let Some(response) = error.as_error_resp() {
                    match response.code {
                        429 | -32005 | -32016 => ChainError::RateLimited {
                            retry_after_ms: None,
                        },
                        code => ChainError::Rpc { code },
                    }
                } else if error.is_transport_error() {
                    ChainError::Transport("RPC transport failed".into())
                } else {
                    ChainError::Decode("invalid JSON-RPC response".into())
                }
            })
    }
}

pub(crate) fn quantity(value: &Value) -> Result<u64> {
    let raw = value
        .as_str()
        .and_then(|s| s.strip_prefix("0x"))
        .filter(|s| !s.is_empty() && (s.len() == 1 || !s.starts_with('0')))
        .ok_or_else(|| ChainError::Decode("invalid RPC quantity".into()))?;
    u64::from_str_radix(raw, 16).map_err(|_| ChainError::Decode("RPC quantity exceeds u64".into()))
}
