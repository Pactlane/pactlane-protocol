//! A refund has no deadline: a funded job forgotten for a month after its
//! expiry still returns the full budget to its client.

use pactlane_interfaces::JobState;
use pactlane_tests::{Setup, JOB_DURATION, STARTING_BALANCE};

const THIRTY_DAYS: u64 = 30 * 24 * 60 * 60;

#[test]
fn a_refund_is_still_claimable_thirty_days_after_expiry() {
    let s = Setup::new();
    let kernel = s.kernel();
    let funded = s.funded_job(&kernel);
    let submitted = s.submitted_job(&kernel);

    s.advance_time(JOB_DURATION + THIRTY_DAYS);

    for id in [funded, submitted] {
        kernel.claim_refund(&id);
        assert_eq!(kernel.get_job(&id).state, JobState::Expired);
    }
    assert_eq!(s.balance(&s.client), STARTING_BALANCE);
    assert_eq!(s.balance(&kernel.address), 0);
    assert_eq!(s.balance(&s.provider), 0);
}
