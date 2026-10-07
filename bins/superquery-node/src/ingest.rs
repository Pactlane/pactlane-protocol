use crate::{shutdown::Shutdown, startup::NodeContext};
use superquery_chain_api::{ChainAdapter, GenericBlock};
use superquery_chain_evm::{project::PreparedHandler, EvmBlock};
use superquery_config::NodeConfig;
use superquery_core::fetch::scheduler::BlockSink;
use superquery_core::{Backpressure, FetchScheduler, RangePlan, SchedulerConfig};
use superquery_store::{Database, IngestionStore};

struct Sink {
    db: Database,
    journal: IngestionStore,
    handlers: Vec<PreparedHandler>,
}

#[async_trait::async_trait]
impl BlockSink<GenericBlock<EvmBlock>> for Sink {
    async fn accept(&mut self, block: GenericBlock<EvmBlock>) -> superquery_core::Result<()> {
        let matching_inputs: usize = self
            .handlers
            .iter()
            .map(|h| h.matching_inputs(&block.inner))
            .sum();
        self.journal.commit(&self.db, &block.header).await?;
        tracing::info!(height = block.header.height, hash = %block.header.hash, matching_inputs, "finalized block ingested; mappings not executed");
        Ok(())
    }
}

pub async fn run(
    config: &NodeConfig,
    context: NodeContext,
    shutdown: Shutdown,
) -> anyhow::Result<()> {
    context.resume_position().await?;
    let journal = IngestionStore::new(&context.db_config.schema)?;
    journal.ensure_table(&context.database).await?;
    let start_height = config
        .start_height
        .unwrap_or_else(|| context.project.manifest.min_start_block());
    let mut scheduler = FetchScheduler::new(
        context.adapter.clone(),
        SchedulerConfig {
            start_height,
            end_height: config.end_height,
            batch_size: config.batch_size,
            index_unfinalized: false,
            finality_confirmations: config.finality_confirmations,
            backpressure: Backpressure::new(config.max_in_flight, config.queue_capacity),
        },
        RangePlan::new(),
    );
    if let Some(last) = journal.load(&context.database).await? {
        let canonical = tokio::select! {
            biased;
            _ = shutdown.recv() => return Ok(()),
            result = context.adapter.header_at(last.height) => result?,
        };
        anyhow::ensure!(
            canonical.hash == last.hash,
            "ingestion checkpoint is no longer canonical; rewind is not yet available"
        );
        anyhow::ensure!(
            start_height <= last.height.saturating_add(1),
            "start height would skip the persisted ingestion cursor"
        );
        scheduler.resume_from(last.height);
        tracing::info!(height = last.height, "resuming ingestion cursor");
    }
    let mut sink = Sink {
        db: context.database,
        journal,
        handlers: context.handlers,
    };
    scheduler.run(&mut sink, shutdown.token()).await?;
    Ok(())
}
