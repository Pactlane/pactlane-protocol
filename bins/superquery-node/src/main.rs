//! # superquery-node
//!
//! The indexer binary. Guide Milestone 1's acceptance criteria, in order:
//!
//! 1. parse config
//! 2. connect Postgres
//! 3. connect RPC
//! 4. load project
//! 5. print chain/project info
//! 6. shut down cleanly on SIGINT/SIGTERM
//!
//! Startup is deliberately fail-fast. Every check that can be made before
//! indexing begins is made there, because a misconfiguration discovered thirty
//! seconds in is a misconfiguration discovered after the node has already written
//! to someone's database.

mod ingest;
mod shutdown;
mod startup;

use anyhow::Context;
use clap::Parser;
use superquery_config::NodeConfig;

use crate::shutdown::Shutdown;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // A .env file is a convenience for local development; absence is not an error.
    let _ = dotenvy::dotenv();

    let config = NodeConfig::parse();
    startup::init_tracing(&config)?;

    config
        .validate()
        .map_err(anyhow::Error::msg)
        .context("invalid configuration")?;

    let shutdown = Shutdown::new();
    tokio::spawn(shutdown::listen_for_signals(shutdown.clone()));

    match run(&config, shutdown).await {
        Ok(()) => {
            tracing::info!("shutdown complete");
            Ok(())
        }
        Err(e) => {
            // The chain of causes is where the actual problem usually is, so
            // render it rather than only the top-level message.
            tracing::error!(error = ?e, "node stopped");
            Err(e)
        }
    }
}

async fn run(config: &NodeConfig, shutdown: Shutdown) -> anyhow::Result<()> {
    anyhow::ensure!(
        config.ingest_only,
        "mapping execution is not yet available; use --ingest-only for finalized block ingestion"
    );
    let context = tokio::select! {
        biased;
        _ = shutdown.recv() => return Ok(()),
        result = startup::bootstrap(config) => result?,
    };
    context.log_summary();
    ingest::run(config, context, shutdown).await
}
