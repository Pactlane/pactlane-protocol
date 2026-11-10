//! Each kernel deployment keeps its own jobs, counter and escrow, even when
//! two of them escrow the same token. Job IDs overlap across kernels, so a
//! shared key would let one kernel settle the other's money.

use pactlane_interfaces::{Error, JobState};
use pactlane_tests::{Setup, BUDGET, STARTING_BALANCE};

#[test]
fn two_kernels_on_one_token_share_no_jobs_or_funds() {
    let s = Setup::new();
    let a = s.kernel();
    let b = s.kernel();
    assert_ne!(a.address, b.address);

    let in_a = s.submitted_job(&a);
    s.open_job(&a);
    let in_b = s.funded_job(&b);
    // Both kernels number from 1, so the same ID names different jobs.
    assert_eq!((in_a, in_b), (1, 1));
    assert_eq!((a.job_count(), b.job_count()), (2, 1));
    assert_eq!(s.balance(&a.address), BUDGET);
    assert_eq!(s.balance(&b.address), BUDGET);

    a.complete(&in_a, &s.hash(9));

    assert_eq!(a.get_job(&in_a).state, JobState::Completed);
    assert_eq!(b.get_job(&in_b).state, JobState::Funded);
    assert_eq!(s.balance(&a.address), 0);
    assert_eq!(s.balance(&b.address), BUDGET);
    // Job 2 exists only in kernel A.
    assert_eq!(b.try_get_job(&2), Err(Ok(Error::NotFound)));

    b.reject(&in_b, &None);
    assert_eq!(a.get_job(&in_a).state, JobState::Completed);
    assert_eq!(s.balance(&s.provider), BUDGET);
    assert_eq!(s.balance(&s.client), STARTING_BALANCE - BUDGET);
}
