use super::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use superquery_chain_api::{ChainError, Filter, GenericBlock, Header};
use tokio_util::sync::CancellationToken;

struct Fake {
    active: AtomicUsize,
    peak: AtomicUsize,
    fail: AtomicBool,
    head: u64,
}

#[async_trait::async_trait]
impl ChainAdapter for Fake {
    type Block = ();
    type Event = ();
    type FetchedBlock = GenericBlock<()>;
    fn network_id(&self) -> &str {
        "fake"
    }
    async fn validate_network(&self, _: &str) -> superquery_chain_api::Result<()> {
        Ok(())
    }
    async fn latest_height(&self) -> superquery_chain_api::Result<u64> {
        Ok(self.head)
    }
    async fn finalized_height(&self) -> superquery_chain_api::Result<u64> {
        Ok(self.head)
    }
    async fn header_at(&self, height: u64) -> superquery_chain_api::Result<Header> {
        Ok(Header {
            height,
            hash: format!("h{height}"),
            parent_hash: height.checked_sub(1).map(|h| format!("h{h}")),
            timestamp: None,
        })
    }
    async fn fetch_block(&self, height: u64) -> superquery_chain_api::Result<Self::FetchedBlock> {
        if self.fail.swap(false, Ordering::SeqCst) {
            return Err(ChainError::Transport("temporary".into()));
        }
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(active, Ordering::SeqCst);
        tokio::time::sleep(std::time::Duration::from_micros((8 - height % 8) * 10)).await;
        self.active.fetch_sub(1, Ordering::SeqCst);
        Ok(GenericBlock::new(self.header_at(height).await?, ()))
    }
    async fn decode_events(&self, _: &()) -> superquery_chain_api::Result<Vec<()>> {
        Ok(vec![])
    }
    fn event_matches(&self, _: &(), _: &Filter) -> bool {
        false
    }
}

fn scheduler(end: u64) -> FetchScheduler<Fake> {
    FetchScheduler::new(
        Arc::new(Fake {
            active: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
            fail: AtomicBool::new(false),
            head: end,
        }),
        SchedulerConfig {
            start_height: 0,
            end_height: Some(end),
            batch_size: 100,
            index_unfinalized: false,
            finality_confirmations: 200,
            backpressure: Backpressure::new(8, 16),
        },
        RangePlan::new(),
    )
}

#[derive(Default)]
struct Sink {
    heights: Vec<u64>,
    fail_at: Option<u64>,
}
#[async_trait::async_trait]
impl BlockSink<GenericBlock<()>> for Sink {
    async fn accept(&mut self, block: GenericBlock<()>) -> Result<()> {
        if self.fail_at == Some(block.header.height) {
            return Err(crate::CoreError::Other("commit failed".into()));
        }
        self.heights.push(block.header.height);
        Ok(())
    }
}

#[tokio::test]
async fn thousand_blocks_are_ordered_and_bounded() {
    let mut scheduler = scheduler(999);
    let mut sink = Sink::default();
    scheduler
        .run(&mut sink, &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(sink.heights, (0..1000).collect::<Vec<_>>());
    assert!(scheduler.adapter.peak.load(Ordering::SeqCst) <= 8);
    assert!(scheduler.adapter.peak.load(Ordering::SeqCst) > 1);
    assert!(scheduler.is_complete());
}

#[tokio::test]
async fn failed_fetch_does_not_skip_work() {
    let mut scheduler = scheduler(20);
    scheduler.adapter.fail.store(true, Ordering::SeqCst);
    assert!(scheduler.fetch_next().await.is_err());
    assert_eq!(scheduler.next_height(), 0);
    let blocks = scheduler.fetch_next().await.unwrap();
    assert_eq!(blocks.first().unwrap().header.height, 0);
    assert!(scheduler.acknowledge(1).is_err());
}

#[tokio::test]
async fn commit_failure_resumes_at_first_uncommitted_block() {
    let mut scheduler = scheduler(20);
    let mut sink = Sink {
        fail_at: Some(3),
        ..Default::default()
    };
    assert!(scheduler
        .run(&mut sink, &CancellationToken::new())
        .await
        .is_err());
    assert_eq!(scheduler.next_height(), 3);
    sink.fail_at = None;
    scheduler
        .run(&mut sink, &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(sink.heights, (0..=20).collect::<Vec<_>>());
}

#[tokio::test]
async fn cancellation_is_sticky_and_max_height_exhausts() {
    let mut scheduler = scheduler(u64::MAX);
    let cancel = CancellationToken::new();
    cancel.cancel();
    let mut sink = Sink::default();
    scheduler.run(&mut sink, &cancel).await.unwrap();
    assert!(sink.heights.is_empty());
    scheduler.resume_from(u64::MAX - 1);
    assert_eq!(scheduler.next_heights(0, 0).await.unwrap(), vec![u64::MAX]);
    scheduler.acknowledge(u64::MAX).unwrap();
    assert!(scheduler.is_complete());
    assert!(scheduler.next_heights(0, 0).await.unwrap().is_empty());
}

#[tokio::test]
async fn trailing_bypass_completes_and_zero_batch_is_rejected() {
    let mut s = scheduler(20);
    s.plan = RangePlan::new().bypass(0, 20);
    assert!(s.is_complete());
    let mut s = scheduler(20);
    s.config.batch_size = 0;
    assert!(s
        .run(&mut Sink::default(), &CancellationToken::new())
        .await
        .is_err());
}
