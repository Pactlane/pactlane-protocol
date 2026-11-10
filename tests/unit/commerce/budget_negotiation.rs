//! Client and provider may counter-propose as often as they like while the
//! job is open. Only the latest proposal counts, and `fund` takes exactly it.

use pactlane_interfaces::{Error, JobState};
use pactlane_tests::{Setup, STARTING_BALANCE, USDC};

#[test]
fn after_twenty_alternating_proposals_only_the_last_can_be_funded() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job(&kernel);

    let proposals: std::vec::Vec<i128> = (1..=20).map(|round| round * USDC + round).collect();
    for (round, amount) in proposals.iter().enumerate() {
        let actor = if round % 2 == 0 {
            &s.client
        } else {
            &s.provider
        };
        kernel.set_budget(&id, actor, amount);
        assert_eq!(kernel.get_job(&id).budget, *amount, "round {round}");
    }
    let last = *proposals.last().unwrap();

    for earlier in &proposals[..proposals.len() - 1] {
        assert_eq!(
            kernel.try_fund(&id, earlier),
            Err(Ok(Error::BudgetMismatch)),
            "funding the superseded proposal {earlier}"
        );
    }
    assert_eq!(s.balance(&s.client), STARTING_BALANCE);

    kernel.fund(&id, &last);

    assert_eq!(kernel.get_job(&id).state, JobState::Funded);
    assert_eq!(s.balance(&kernel.address), last);
    assert_eq!(s.balance(&s.client), STARTING_BALANCE - last);
}
