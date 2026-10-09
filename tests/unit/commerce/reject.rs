use pactlane_interfaces::{Error, JobRejected, JobState, Refunded};
use pactlane_tests::{Setup, BUDGET, JOB_DURATION, STARTING_BALANCE};
use soroban_sdk::Event;

#[test]
fn the_client_cancels_an_open_job_and_nothing_moves() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job(&kernel);

    kernel.reject(&id, &None);

    s.assert_authorized_by(&s.client, &kernel.address, "reject");
    assert_eq!(
        s.events_of(&kernel.address),
        [JobRejected {
            id,
            rejector: s.client.clone(),
            reason: None
        }
        .to_xdr(&s.env, &kernel.address)]
    );
    assert_eq!(kernel.get_job(&id).state, JobState::Rejected);
    assert_eq!(s.balance(&s.client), STARTING_BALANCE);
}

#[test]
fn an_open_job_can_be_cancelled_even_after_it_expires() {
    // An open job never expires on its own; without this it would be stuck.
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job(&kernel);

    s.advance_time(JOB_DURATION * 2);
    kernel.reject(&id, &None);

    assert_eq!(kernel.get_job(&id).state, JobState::Rejected);
}

#[test]
fn the_evaluator_rejects_funded_or_submitted_work_and_the_client_is_refunded() {
    let s = Setup::new();
    let kernel = s.kernel();
    let funded = s.funded_job(&kernel);
    let submitted = s.submitted_job(&kernel);

    for id in [funded, submitted] {
        kernel.reject(&id, &Some(s.hash(8)));

        s.assert_authorized_by(&s.evaluator, &kernel.address, "reject");
        assert_eq!(
            s.events_of(&kernel.address),
            [
                JobRejected {
                    id,
                    rejector: s.evaluator.clone(),
                    reason: Some(s.hash(8))
                }
                .to_xdr(&s.env, &kernel.address),
                Refunded {
                    id,
                    client: s.client.clone(),
                    amount: BUDGET
                }
                .to_xdr(&s.env, &kernel.address),
            ]
        );
        let job = kernel.get_job(&id);
        assert_eq!(job.state, JobState::Rejected);
        assert_eq!(job.reason, Some(s.hash(8)));
    }
    assert_eq!(s.balance(&s.client), STARTING_BALANCE);
    assert_eq!(s.balance(&kernel.address), 0);
    assert_eq!(s.balance(&s.provider), 0);
}

#[test]
fn a_rejected_job_stays_rejected() {
    // T6: once refunded, nothing may pay out or refund again.
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.submitted_job(&kernel);
    kernel.reject(&id, &None);

    assert_eq!(kernel.try_reject(&id, &None), Err(Ok(Error::BadState)));
    assert_eq!(
        kernel.try_complete(&id, &s.hash(9)),
        Err(Ok(Error::BadState))
    );
    assert_eq!(s.balance(&s.client), STARTING_BALANCE);
    assert_eq!(s.balance(&s.provider), 0);
}

#[test]
fn a_cancelled_job_cannot_be_revived() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job(&kernel);
    kernel.set_budget(&id, &s.client, &BUDGET);
    kernel.reject(&id, &None);

    assert_eq!(kernel.try_fund(&id, &BUDGET), Err(Ok(Error::BadState)));
    assert_eq!(
        kernel.try_set_budget(&id, &s.client, &BUDGET),
        Err(Ok(Error::BadState))
    );
}

#[test]
fn a_completed_job_cannot_be_rejected() {
    // T6: rejecting after payout would refund the client a second budget.
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.submitted_job(&kernel);
    let other = s.funded_job(&kernel);
    kernel.complete(&id, &s.hash(9));

    assert_eq!(kernel.try_reject(&id, &None), Err(Ok(Error::BadState)));
    assert_eq!(s.balance(&kernel.address), BUDGET);
    assert_eq!(kernel.get_job(&other).state, JobState::Funded);
}

#[test]
fn the_evaluator_cannot_reject_once_the_job_expires() {
    // T7: after expiry the refund belongs to claim_refund alone.
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.submitted_job(&kernel);

    s.advance_time(JOB_DURATION);

    assert_eq!(kernel.try_reject(&id, &None), Err(Ok(Error::Expired)));
    assert_eq!(s.balance(&kernel.address), BUDGET);
}

#[test]
fn rejecting_an_unknown_job_fails() {
    let s = Setup::new();
    let kernel = s.kernel();

    assert_eq!(kernel.try_reject(&1, &None), Err(Ok(Error::NotFound)));
}
