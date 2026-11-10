//! `expected_budget` is the amount the client agreed to. A negative one can
//! never equal a stored budget, which is always positive by the time a job
//! can be funded, so it must be refused like any other mismatch.

use pactlane_interfaces::{Error, JobState};
use pactlane_tests::{Setup, BUDGET, STARTING_BALANCE};

#[test]
fn funding_with_a_negative_expected_budget_is_refused_and_moves_nothing() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job(&kernel);
    kernel.set_budget(&id, &s.client, &BUDGET);
    let before = kernel.get_job(&id);

    // -BUDGET has the budget's magnitude, and i128::MIN has no positive
    // counterpart, so a comparison on absolute values would fail on both.
    for expected in [-1, -BUDGET, i128::MIN] {
        assert_eq!(
            kernel.try_fund(&id, &expected),
            Err(Ok(Error::BudgetMismatch)),
            "expected_budget = {expected}"
        );
        assert_eq!(kernel.get_job(&id), before, "expected_budget = {expected}");
        assert_eq!(s.balance(&s.client), STARTING_BALANCE);
        assert_eq!(s.balance(&kernel.address), 0);
    }
}

#[test]
fn a_job_refused_for_a_negative_expected_budget_can_still_be_funded() {
    // The refusal leaves nothing behind that blocks the real agreement.
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job(&kernel);
    kernel.set_budget(&id, &s.client, &BUDGET);
    let _ = kernel.try_fund(&id, &-BUDGET);

    kernel.fund(&id, &BUDGET);

    assert_eq!(kernel.get_job(&id).state, JobState::Funded);
    assert_eq!(s.balance(&s.client), STARTING_BALANCE - BUDGET);
    assert_eq!(s.balance(&kernel.address), BUDGET);
}
