use pactlane_interfaces::{Error, JobCompleted, JobState, PaymentReleased};
use pactlane_tests::{Setup, BUDGET, JOB_DURATION, STARTING_BALANCE};
use soroban_sdk::Event;

#[test]
fn completion_pays_the_provider_the_full_budget() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.submitted_job(&kernel);

    kernel.complete(&id, &s.hash(9));

    assert_eq!(s.balance(&s.provider), BUDGET);
    assert_eq!(s.balance(&kernel.address), 0);
    assert_eq!(s.balance(&s.client), STARTING_BALANCE - BUDGET);
    // The evaluator decides where the money goes but never receives any.
    assert_eq!(s.balance(&s.evaluator), 0);
}

#[test]
fn only_the_evaluator_authorizes_completion() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.submitted_job(&kernel);

    kernel.complete(&id, &s.hash(9));

    s.assert_authorized_by(&s.evaluator, &kernel.address, "complete");
}

#[test]
fn completion_emits_job_completed_then_payment_released() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.submitted_job(&kernel);

    kernel.complete(&id, &s.hash(9));

    assert_eq!(
        s.events_of(&kernel.address),
        [
            JobCompleted {
                id,
                evaluator: s.evaluator.clone(),
                reason: s.hash(9)
            }
            .to_xdr(&s.env, &kernel.address),
            PaymentReleased {
                id,
                provider: s.provider.clone(),
                amount: BUDGET
            }
            .to_xdr(&s.env, &kernel.address),
        ]
    );
}

#[test]
fn completion_records_the_evidence_hash() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.submitted_job(&kernel);

    kernel.complete(&id, &s.hash(9));

    let job = kernel.get_job(&id);
    assert_eq!(job.state, JobState::Completed);
    assert_eq!(job.reason, Some(s.hash(9)));
}

#[test]
fn only_submitted_work_can_be_completed() {
    // Paying for work that was never submitted pays for nothing.
    let s = Setup::new();
    let kernel = s.kernel();
    let open = s.open_job(&kernel);
    let funded = s.funded_job(&kernel);

    for (id, state) in [(open, "open"), (funded, "funded")] {
        assert_eq!(
            kernel.try_complete(&id, &s.hash(9)),
            Err(Ok(Error::BadState)),
            "{state}"
        );
    }
    assert_eq!(s.balance(&s.provider), 0);
}

#[test]
fn a_job_is_paid_only_once() {
    // T6: a second completion would pay the provider from other jobs' escrow.
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.submitted_job(&kernel);
    let other = s.funded_job(&kernel);
    kernel.complete(&id, &s.hash(9));

    assert_eq!(
        kernel.try_complete(&id, &s.hash(9)),
        Err(Ok(Error::BadState))
    );
    assert_eq!(s.balance(&s.provider), BUDGET);
    assert_eq!(kernel.get_job(&other).state, JobState::Funded);
    assert_eq!(s.balance(&kernel.address), BUDGET);
}

#[test]
fn completion_is_refused_once_the_job_expires() {
    // T7: after expiry only a refund is possible, so payout and refund never race.
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.submitted_job(&kernel);

    s.advance_time(JOB_DURATION);

    assert_eq!(
        kernel.try_complete(&id, &s.hash(9)),
        Err(Ok(Error::Expired))
    );
    assert_eq!(s.balance(&kernel.address), BUDGET);
}

#[test]
fn completing_an_unknown_job_fails() {
    let s = Setup::new();
    let kernel = s.kernel();

    assert_eq!(
        kernel.try_complete(&1, &s.hash(9)),
        Err(Ok(Error::NotFound))
    );
}
