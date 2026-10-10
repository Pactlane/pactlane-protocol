use pactlane_interfaces::{Error, JobState};
use pactlane_tests::{Setup, USDC};
#[test]
fn the_last_budget_proposal_wins_and_fund_requires_exactly_it() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job(&kernel);
    let mut last = 0;
    let mut previous = 0;

    for round in 0..20 {
        let amount = if round % 2 == 0 {
            5 * USDC + round
        } else {
            4 * USDC + round
        };
        let actor = if round % 2 == 0 {
            &s.client
        } else {
            &s.provider
        };

        kernel.set_budget(&id, actor, &amount);
        if round == 18 {
            previous = amount;
        };
        last = amount;
    }

    assert_ne!(last, previous);
    assert_eq!(kernel.get_job(&id).budget, last);

    assert_eq!(
        kernel.try_fund(&id, &previous),
        Err(Ok(Error::BudgetMismatch))
    );
    assert_eq!(kernel.get_job(&id).state, JobState::Open);

    kernel.fund(&id, &last);
    assert_eq!(kernel.get_job(&id).state, JobState::Funded);
}
