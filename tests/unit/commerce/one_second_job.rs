//! The shortest job: `expires_at = now + 1` is live for exactly the creation
//! second and expired from the next one.

use pactlane_interfaces::{Error, JobState};
use pactlane_tests::{Setup, BUDGET, STARTING_BALANCE};

#[test]
fn a_one_second_job_can_be_funded_in_its_only_live_second() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = kernel.create_job(
        &s.client,
        &Some(s.provider.clone()),
        &s.evaluator,
        &(s.now() + 1),
        &s.hash(1),
    );
    kernel.set_budget(&id, &s.client, &BUDGET);

    kernel.fund(&id, &BUDGET);

    assert_eq!(kernel.get_job(&id).state, JobState::Funded);
}

#[test]
fn a_one_second_job_is_expired_one_second_later() {
    let s = Setup::new();
    let kernel = s.kernel();
    let later = s.now() + 1;
    let unfunded = kernel.create_job(
        &s.client,
        &Some(s.provider.clone()),
        &s.evaluator,
        &later,
        &s.hash(1),
    );
    kernel.set_budget(&unfunded, &s.client, &BUDGET);
    let funded = kernel.create_job(
        &s.client,
        &Some(s.provider.clone()),
        &s.evaluator,
        &later,
        &s.hash(1),
    );
    kernel.set_budget(&funded, &s.client, &BUDGET);
    kernel.fund(&funded, &BUDGET);
    assert_eq!(kernel.try_claim_refund(&funded), Err(Ok(Error::NotExpired)));

    s.advance_time(1);

    assert_eq!(kernel.try_fund(&unfunded, &BUDGET), Err(Ok(Error::Expired)));
    assert_eq!(
        kernel.try_submit(&funded, &s.hash(2)),
        Err(Ok(Error::Expired))
    );
    kernel.claim_refund(&funded);
    assert_eq!(kernel.get_job(&funded).state, JobState::Expired);
    assert_eq!(s.balance(&s.client), STARTING_BALANCE);
}
