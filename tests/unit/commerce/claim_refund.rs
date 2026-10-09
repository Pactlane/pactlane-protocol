use pactlane_interfaces::{Error, JobExpired, JobState, Refunded};
use pactlane_tests::{Setup, BUDGET, JOB_DURATION, STARTING_BALANCE};
use soroban_sdk::Event;

#[test]
fn after_expiry_a_funded_or_submitted_job_refunds_its_client() {
    // T8: an evaluator who never responds cannot strand the client's money.
    let s = Setup::new();
    let kernel = s.kernel();
    let funded = s.funded_job(&kernel);
    let submitted = s.submitted_job(&kernel);

    s.advance_time(JOB_DURATION);

    for id in [funded, submitted] {
        kernel.claim_refund(&id);
        assert_eq!(kernel.get_job(&id).state, JobState::Expired);
    }
    assert_eq!(s.balance(&s.client), STARTING_BALANCE);
    assert_eq!(s.balance(&kernel.address), 0);
    assert_eq!(s.balance(&s.provider), 0);
}

#[test]
fn claiming_a_refund_needs_no_signature() {
    // Whoever submits the transaction (here, nobody in particular) needs no
    // role, so the refund never depends on one party being online.
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.funded_job(&kernel);
    s.advance_time(JOB_DURATION);

    kernel.claim_refund(&id);

    assert_eq!(s.env.auths(), std::vec![]);
}

#[test]
fn a_refund_emits_job_expired_then_refunded() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.funded_job(&kernel);
    s.advance_time(JOB_DURATION);

    kernel.claim_refund(&id);

    assert_eq!(
        s.events_of(&kernel.address),
        [
            JobExpired { id }.to_xdr(&s.env, &kernel.address),
            Refunded {
                id,
                client: s.client.clone(),
                amount: BUDGET
            }
            .to_xdr(&s.env, &kernel.address),
        ]
    );
}

#[test]
fn a_refund_cannot_be_claimed_while_the_job_is_live() {
    // Claiming early would cancel funded work behind the evaluator's back.
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.submitted_job(&kernel);

    s.advance_time(JOB_DURATION - 1);

    assert_eq!(kernel.try_claim_refund(&id), Err(Ok(Error::NotExpired)));
    assert_eq!(s.balance(&kernel.address), BUDGET);
}

#[test]
fn a_refund_is_paid_only_once() {
    // T6: a second claim would refund the client from other jobs' escrow.
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.funded_job(&kernel);
    let other = s.funded_job(&kernel);
    s.advance_time(JOB_DURATION);
    kernel.claim_refund(&id);

    assert_eq!(kernel.try_claim_refund(&id), Err(Ok(Error::BadState)));
    assert_eq!(s.balance(&kernel.address), BUDGET);
    assert_eq!(kernel.get_job(&other).state, JobState::Funded);
}

#[test]
fn jobs_holding_no_funds_have_nothing_to_refund() {
    let s = Setup::new();
    let kernel = s.kernel();
    let open = s.open_job(&kernel);
    let completed = s.submitted_job(&kernel);
    kernel.complete(&completed, &s.hash(9));
    let rejected = s.funded_job(&kernel);
    kernel.reject(&rejected, &None);

    s.advance_time(JOB_DURATION);

    for (id, state) in [
        (open, "open"),
        (completed, "completed"),
        (rejected, "rejected"),
    ] {
        assert_eq!(
            kernel.try_claim_refund(&id),
            Err(Ok(Error::BadState)),
            "{state}"
        );
    }
}

#[test]
fn claiming_a_refund_for_an_unknown_job_fails() {
    let s = Setup::new();
    let kernel = s.kernel();

    assert_eq!(kernel.try_claim_refund(&1), Err(Ok(Error::NotFound)));
}
