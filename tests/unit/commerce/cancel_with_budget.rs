//! A proposed budget is only a number on the job: no tokens move and no
//! allowance is granted until `fund`. Cancelling an open job with a budget
//! therefore leaves the client exactly as it was.

use pactlane_interfaces::JobState;
use pactlane_tests::{Setup, BUDGET, STARTING_BALANCE};

#[test]
fn cancelling_an_open_job_with_a_budget_moves_no_tokens() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job(&kernel);
    kernel.set_budget(&id, &s.provider, &BUDGET);
    kernel.set_budget(&id, &s.client, &BUDGET);

    kernel.reject(&id, &None);

    assert_eq!(kernel.get_job(&id).state, JobState::Rejected);
    assert_eq!(s.balance(&s.client), STARTING_BALANCE);
    assert_eq!(s.token.allowance(&s.client, &kernel.address), 0);
    assert_eq!(s.balance(&kernel.address), 0);
    assert_eq!(s.balance(&s.provider), 0);
}
