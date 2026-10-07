//! Bounded block fetching with acknowledgement-based progress.
use std::sync::Arc;

use superquery_chain_api::{ChainAdapter, IBlock};
use futures::{stream, StreamExt, TryStreamExt};

use crate::error::Result;
use crate::fetch::backpressure::Backpressure;
use crate::fetch::range::{safe_head, RangePlan};

/// Runtime settings for one fetch loop.
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// First height to index.
    pub start_height: u64,
    /// Last height to index, if the run is bounded.
    pub end_height: Option<u64>,
    /// Blocks per batch.
    pub batch_size: u32,
    /// Whether to index past the finalized height.
    pub index_unfinalized: bool,
    /// Depth treated as final on chains without a finality gadget.
    pub finality_confirmations: u64,
    /// Concurrency and queue limits.
    pub backpressure: Backpressure,
}

/// Drives block fetching for one chain.
pub struct FetchScheduler<A: ChainAdapter> {
    adapter: Arc<A>,
    config: SchedulerConfig,
    plan: RangePlan,
    next_height: u64,
    exhausted: bool,
    pending: std::collections::VecDeque<u64>,
}

impl<A: ChainAdapter> FetchScheduler<A> {
    /// Create a scheduler positioned at `config.start_height`.
    pub fn new(adapter: Arc<A>, config: SchedulerConfig, plan: RangePlan) -> Self {
        let next_height = config.start_height;
        Self {
            adapter,
            config,
            plan,
            next_height,
            exhausted: false,
            pending: Default::default(),
        }
    }

    /// The next height not yet dispatched.
    pub fn next_height(&self) -> u64 {
        self.next_height
    }

    /// Resume from a checkpoint: continue at the block after the last committed one.
    pub fn resume_from(&mut self, last_indexed: u64) {
        self.exhausted = last_indexed == u64::MAX;
        self.next_height = last_indexed.saturating_add(1).max(self.config.start_height);
        self.pending.clear();
    }

    /// Ask the chain how far it is currently safe to index.
    pub async fn safe_head(&self) -> Result<u64> {
        let latest = self.adapter.latest_height().await?;
        let finalized = self.adapter.finalized_height().await?;
        Ok(safe_head(
            latest,
            finalized,
            self.config.index_unfinalized,
            self.config.finality_confirmations,
        ))
    }

    /// Plan a batch without advancing past unacknowledged work.
    pub async fn next_heights(&mut self, in_flight: u32, queued: u32) -> Result<Vec<u64>> {
        if self.is_complete() { return Ok(Vec::new()); }
        let budget = self.config.backpressure.clamp_batch(self.config.batch_size, in_flight, queued);
        if budget == 0 { return Ok(Vec::new()); }
        if !self.pending.is_empty() { return Ok(self.pending.iter().copied().take(budget as usize).collect()); }
        let head = self.safe_head().await?;
        let Some(range) = self.plan.next_batch(self.next_height, head, budget, self.config.end_height) else {
            return Ok(Vec::new());
        };
        self.pending = self.plan.heights_in(&range).into();
        Ok(self.pending.iter().copied().collect())
    }

    /// Advance only after the consumer durably accepts this exact next block.
    pub fn acknowledge(&mut self, height: u64) -> Result<()> {
        if self.pending.front() != Some(&height) {
            return Err(crate::CoreError::Other("out-of-order scheduler acknowledgement".into()));
        }
        self.pending.pop_front();
        self.exhausted = height == u64::MAX;
        self.next_height = height.saturating_add(1);
        Ok(())
    }

    /// Whether the configured finite range has been consumed.
    pub fn is_complete(&self) -> bool {
        self.exhausted || (self.pending.is_empty() && self.config.end_height.is_some_and(|end| {
            self.plan.next_batch(self.next_height, end, self.config.batch_size, Some(end)).is_none()
        }))
    }

    /// Fetch concurrently while bounding retained blocks and preserving height order.
    pub async fn fetch_next(&mut self) -> Result<Vec<A::FetchedBlock>> {
        let heights = self.next_heights(0, 0).await?;
        let adapter = &self.adapter;
        let blocks: Vec<A::FetchedBlock> = stream::iter(heights.into_iter().map(|height| async move {
            let block = adapter.fetch_block(height).await?;
            if block.header().height != height {
                return Err(crate::CoreError::Other("adapter returned the wrong block height".into()));
            }
            Ok(block)
        })).buffered(self.config.backpressure.max_in_flight.max(1) as usize).try_collect().await?;
        for pair in blocks.windows(2) {
            if !pair[1].header().is_child_of(pair[0].header()) {
                return Err(crate::CoreError::Other("fetched batch contains inconsistent parent hashes".into()));
            }
        }
        Ok(blocks)
    }

    /// Stop fetching promptly on cancellation; finish any active durable accept.
    pub async fn run<S: BlockSink<A::FetchedBlock>>(
        &mut self, sink: &mut S, cancel: &tokio_util::sync::CancellationToken,
    ) -> Result<()> {
        if self.config.batch_size == 0 {
            return Err(crate::CoreError::Other("scheduler batch size must be positive".into()));
        }
        while !self.is_complete() {
            let blocks = tokio::select! {
                biased;
                _ = cancel.cancelled() => return Ok(()),
                result = self.fetch_next() => result?,
            };
            if blocks.is_empty() {
                tokio::select! {
                    biased;
                    _ = cancel.cancelled() => return Ok(()),
                    _ = tokio::time::sleep(std::time::Duration::from_millis(self.adapter.block_interval_ms().max(1))) => {},
                }
                continue;
            }
            for block in blocks {
                if cancel.is_cancelled() { return Ok(()); }
                let height = block.header().height;
                sink.accept(block).await?;
                self.acknowledge(height)?;
            }
        }
        Ok(())
    }
}

/// A consumer must return success only after the block is durably accepted.
#[async_trait::async_trait]
pub trait BlockSink<B: Send>: Send {
    /// Commit the next block or return an error without acknowledging it.
    async fn accept(&mut self, block: B) -> Result<()>;
}
