use crate::{EvmAdapter, EvmAdapterConfig};
use axum::{routing::post, Json, Router};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use superquery_chain_api::{ChainAdapter, ChainError};

struct Mock {
    url: String,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Mock {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn mock(f: impl Fn(Value) -> Value + Send + Sync + 'static) -> Mock {
    let f = Arc::new(f);
    let router = Router::new().route(
        "/",
        post(move |Json(request): Json<Value>| {
            let f = f.clone();
            async move {
                let result = f(request.clone());
                let mut response = json!({"jsonrpc":"2.0", "id":request["id"]});
                if result.get("error").is_some() {
                    response["error"] = result["error"].clone();
                } else {
                    response["result"] = result;
                }
                Json(response)
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    Mock { url, task }
}
fn adapter(urls: Vec<String>, attempts: u32) -> EvmAdapter {
    EvmAdapter::new(EvmAdapterConfig {
        endpoints: urls,
        max_retries: attempts,
        timeout_secs: 1,
        ..Default::default()
    })
}

#[tokio::test]
async fn retries_rate_limits_and_verifies_failover_chain() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let count = attempts.clone();
    let server = mock(move |request| match request["method"].as_str().unwrap() {
        "eth_chainId" => json!("0x1"),
        "eth_blockNumber" => {
            if count.fetch_add(1, Ordering::SeqCst) == 0 {
                json!({"error":{"code":-32005,"message":"rate limit"}})
            } else {
                json!("0x10")
            }
        }
        _ => panic!("unexpected request"),
    })
    .await;
    assert_eq!(
        adapter(vec![server.url.clone()], 2)
            .latest_height()
            .await
            .unwrap(),
        16
    );
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    let wrong = mock(|_| json!("0x89")).await;
    let unavailable = mock(|_| json!({"error":{"code":-32005,"message":"busy"}})).await;
    assert!(matches!(
        adapter(vec![unavailable.url.clone(), wrong.url.clone()], 2)
            .latest_height()
            .await,
        Err(ChainError::ChainIdMismatch { .. })
    ));
}

#[tokio::test]
async fn fatal_rpc_errors_are_not_retried_and_finality_fallback_is_narrow() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let count = attempts.clone();
    let server = mock(move |request| match request["method"].as_str().unwrap() {
        "eth_chainId" => json!("0x1"),
        _ => {
            count.fetch_add(1, Ordering::SeqCst);
            json!({"error":{"code":-32600,"message":"bad request"}})
        }
    })
    .await;
    assert!(adapter(vec![server.url.clone()], 3)
        .finalized_height()
        .await
        .is_err());
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
    let server = mock(|request| match request["method"].as_str().unwrap() {
        "eth_chainId" => json!("0x1"),
        "eth_blockNumber" => json!("0x3e8"),
        _ => json!({"error":{"code":-32602,"message":"unsupported finalized tag"}}),
    })
    .await;
    assert_eq!(
        adapter(vec![server.url.clone()], 1)
            .finalized_height()
            .await
            .unwrap(),
        800
    );
}

#[tokio::test]
async fn fetch_uses_hash_pinned_logs() {
    let hash = format!("0x{}", "ab".repeat(32));
    let expected = hash.clone();
    let server = mock(move |request| match request["method"].as_str().unwrap() {
        "eth_chainId" => json!("0x1"),
        "eth_getBlockByNumber" => json!({"number":"0x2", "hash":hash, "parentHash":format!("0x{}", "00".repeat(32)), "timestamp":"0x1234", "transactions":[]}),
        "eth_getLogs" => { assert_eq!(request["params"][0]["blockHash"], hash); json!([]) },
        _ => panic!("unexpected request"),
    }).await;
    let block = adapter(vec![server.url.clone()], 1)
        .fetch_block(2)
        .await
        .unwrap();
    assert_eq!(block.header.hash, expected);
    assert_eq!(block.header.height, 2);
}

#[tokio::test]
async fn hanging_rpc_is_timed_out() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let (_stream, _) = listener.accept().await.unwrap();
        std::future::pending::<()>().await;
    });
    let result = adapter(vec![url], 1).latest_height().await;
    task.abort();
    assert!(matches!(result, Err(ChainError::Transport(_))));
}
