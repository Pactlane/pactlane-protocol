use pactlane_interfaces::{BudgetSet, Error};
use pactlane_tests::{Setup, JOB_DURATION, USDC};
use soroban_sdk::Event;

#[test]
fn the_client_or_the_provider_can_propose_a_budget() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job(&kernel);

    for (actor, amount) in [(&s.client, 5 * USDC), (&s.provider, 4 * USDC)] {
        kernel.set_budget(&id, actor, &amount);

        s.assert_authorized_by(actor, &kernel.address, "set_budget");
        assert_eq!(
            s.events_of(&kernel.address),
            [BudgetSet {
                id,
                actor: actor.clone(),
                amount
            }
            .to_xdr(&s.env, &kernel.address)]
        );
        assert_eq!(kernel.get_job(&id).budget, amount);
    }
}

#[test]
fn nobody_else_can_propose_a_budget() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job(&kernel);

    for (who, name) in [(&s.evaluator, "evaluator"), (&s.stranger, "stranger")] {
        assert_eq!(
            kernel.try_set_budget(&id, who, &USDC),
            Err(Ok(Error::BadActor)),
            "{name}"
        );
    }
    assert_eq!(kernel.get_job(&id).budget, 0);
}

#[test]
fn a_provider_can_propose_only_once_assigned() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job_without_provider(&kernel);

    assert_eq!(
        kernel.try_set_budget(&id, &s.provider, &USDC),
        Err(Ok(Error::BadActor))
    );

    kernel.set_provider(&id, &s.provider);
    kernel.set_budget(&id, &s.provider, &USDC);
    assert_eq!(kernel.get_job(&id).budget, USDC);
}

#[test]
fn a_budget_must_be_positive() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job(&kernel);

    for amount in [0, -1, i128::MIN] {
        assert_eq!(
            kernel.try_set_budget(&id, &s.client, &amount),
            Err(Ok(Error::BadBudget)),
            "amount {amount}"
        );
    }
}

#[test]
fn no_budget_can_be_proposed_once_the_job_expires() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job(&kernel);

    s.advance_time(JOB_DURATION);

    assert_eq!(
        kernel.try_set_budget(&id, &s.client, &USDC),
        Err(Ok(Error::Expired))
    );
}

#[test]
fn proposing_a_budget_for_an_unknown_job_fails() {
    let s = Setup::new();
    let kernel = s.kernel();

    assert_eq!(
        kernel.try_set_budget(&1, &s.client, &USDC),
        Err(Ok(Error::NotFound))
    );
}
