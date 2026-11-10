//! A job may last exactly `MAX_JOB_DURATION`. At that length it must still
//! be usable end to end, up to the last live second.

use pactlane_interfaces::{Error, JobState, MAX_JOB_DURATION};
use pactlane_tests::{Setup, BUDGET, STARTING_BALANCE};

#[test]
fn a_job_of_exactly_the_maximum_duration_can_be_funded_and_settled() {
    let s = Setup::new();
    let kernel = s.kernel();
    let expires_at = s.now() + MAX_JOB_DURATION;
    let id = kernel.create_job(
        &s.client,
        &Some(s.provider.clone()),
        &s.evaluator,
        &expires_at,
        &s.hash(1),
    );
    kernel.set_budget(&id, &s.client, &BUDGET);
    kernel.fund(&id, &BUDGET);

    // The work arrives and is approved in the last live second.
    s.set_time(expires_at - 1);
    kernel.submit(&id, &s.hash(2));
    kernel.complete(&id, &s.hash(9));

    assert_eq!(kernel.get_job(&id).state, JobState::Completed);
    assert_eq!(s.balance(&s.provider), BUDGET);
    assert_eq!(s.balance(&s.client), STARTING_BALANCE - BUDGET);
    assert_eq!(s.balance(&kernel.address), 0);
}

#[test]
fn a_maximum_duration_job_left_unsettled_is_refunded_at_expiry() {
    let s = Setup::new();
    let kernel = s.kernel();
    let expires_at = s.now() + MAX_JOB_DURATION;
    let id = kernel.create_job(
        &s.client,
        &Some(s.provider.clone()),
        &s.evaluator,
        &expires_at,
        &s.hash(1),
    );
    kernel.set_budget(&id, &s.client, &BUDGET);
    kernel.fund(&id, &BUDGET);
    kernel.submit(&id, &s.hash(2));

    s.set_time(expires_at);
    assert_eq!(
        kernel.try_complete(&id, &s.hash(9)),
        Err(Ok(Error::Expired))
    );
    kernel.claim_refund(&id);

    assert_eq!(kernel.get_job(&id).state, JobState::Expired);
    assert_eq!(s.balance(&s.client), STARTING_BALANCE);
}
