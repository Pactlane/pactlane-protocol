//! Archival and restoration against the deployed Wasm, where the contract's
//! code entry archives along with its instance and jobs.
//!
//! The host restores an archived persistent entry automatically when a call
//! touches it, and `disk_read_entries` counts exactly those restored entries.
//! So archival costs extra reads and rent, but can never strand funds.

use pactlane_interfaces::JobState;
use pactlane_tests::{Setup, BUDGET, JOB_DURATION, STARTING_BALANCE};

use crate::release_wasm;

/// Longer than the network's maximum TTL (about 180 days): everything the
/// kernel wrote before this gap has archived.
const IDLE: u64 = 200 * 24 * 60 * 60;

/// Disk reads of a refund claimed after `wait` seconds. Restored entries are
/// counted as disk reads.
fn refund_reads(wait: u64) -> u32 {
    let s = Setup::new();
    let kernel = s.kernel_from_wasm(&release_wasm("pactlane-commerce"));
    let id = s.funded_job(&kernel);
    s.advance_time(wait);

    kernel.claim_refund(&id);

    let reads = s.env.cost_estimate().resources().disk_read_entries;
    assert_eq!(kernel.get_job(&id).state, JobState::Expired);
    assert_eq!(s.balance(&s.client), STARTING_BALANCE);
    assert_eq!(s.balance(&kernel.address), 0);
    reads
}

#[test]
fn a_funded_job_left_past_the_maximum_ttl_is_still_refunded_in_full() {
    // T12: nobody claimed the refund for 200 days, so the job, the kernel's
    // instance and its code have all archived. The refund must still pay the
    // client in full; restoration is the only extra cost.
    //
    // The baseline is a refund claimed on time. It is not zero: the fixture
    // token's own entries carry the network's minimum TTL, which is 7 days.
    let on_time = refund_reads(JOB_DURATION);
    let idle = refund_reads(IDLE);

    assert!(
        idle >= on_time + 3,
        "expected job, instance and code restored: {idle} reads vs {on_time} on time"
    );
}

#[test]
fn an_archived_finished_job_can_still_be_read_and_renewed() {
    // Indexers and explorers may look up old jobs long after they settled.
    let s = Setup::new();
    let kernel = s.kernel_from_wasm(&release_wasm("pactlane-commerce"));
    let id = s.submitted_job(&kernel);
    kernel.complete(&id, &s.hash(9));
    let finished = kernel.get_job(&id);
    s.advance_time(IDLE);

    kernel.extend_ttl(&id);

    assert!(s.env.cost_estimate().resources().disk_read_entries > 0);
    assert_eq!(kernel.get_job(&id), finished);
    assert_eq!(s.balance(&s.provider), BUDGET);
}
