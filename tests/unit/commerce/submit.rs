use pactlane_interfaces::{Error, JobState, JobSubmitted};
use pactlane_tests::{Setup, BUDGET, JOB_DURATION, STARTING_BALANCE};
use soroban_sdk::Event;

#[test]
fn the_provider_submits_a_work_hash_for_a_funded_job() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.funded_job(&kernel);

    kernel.submit(&id, &s.hash(2));

    s.assert_authorized_by(&s.provider, &kernel.address, "submit");
    assert_eq!(
        s.events_of(&kernel.address),
        [JobSubmitted {
            id,
            provider: s.provider.clone(),
            work_hash: s.hash(2)
        }
        .to_xdr(&s.env, &kernel.address)]
    );
    let job = kernel.get_job(&id);
    assert_eq!(job.state, JobState::Submitted);
    assert_eq!(job.work_hash, Some(s.hash(2)));
}

#[test]
fn submitting_moves_no_funds() {
    let s = Setup::new();
    let kernel = s.kernel();

    s.submitted_job(&kernel);

    assert_eq!(s.balance(&kernel.address), BUDGET);
    assert_eq!(s.balance(&s.client), STARTING_BALANCE - BUDGET);
    assert_eq!(s.balance(&s.provider), 0);
}

#[test]
fn work_cannot_be_submitted_before_funding() {
    // T20: unfunded work is the provider's risk; the kernel will not record it.
    let s = Setup::new();
    let kernel = s.kernel();
    let with_provider = s.open_job(&kernel);
    let without_provider = s.open_job_without_provider(&kernel);

    for id in [with_provider, without_provider] {
        assert_eq!(kernel.try_submit(&id, &s.hash(2)), Err(Ok(Error::BadState)));
    }
}

#[test]
fn work_is_submitted_only_once() {
    // A second submission would let the provider swap the deliverable the
    // evaluator is reviewing.
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.submitted_job(&kernel);

    assert_eq!(kernel.try_submit(&id, &s.hash(3)), Err(Ok(Error::BadState)));
    assert_eq!(kernel.get_job(&id).work_hash, Some(s.hash(2)));
}

#[test]
fn work_cannot_be_submitted_once_the_job_expires() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.funded_job(&kernel);

    s.advance_time(JOB_DURATION);

    assert_eq!(kernel.try_submit(&id, &s.hash(2)), Err(Ok(Error::Expired)));
}

#[test]
fn submitting_to_an_unknown_job_fails() {
    let s = Setup::new();
    let kernel = s.kernel();

    assert_eq!(kernel.try_submit(&1, &s.hash(2)), Err(Ok(Error::NotFound)));
}
