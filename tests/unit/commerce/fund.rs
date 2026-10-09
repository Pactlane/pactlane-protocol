use pactlane_interfaces::{Error, JobFunded, JobState};
use pactlane_tests::{Setup, BUDGET, JOB_DURATION, STARTING_BALANCE, USDC};
use soroban_sdk::{
    testutils::{AuthorizedFunction, AuthorizedInvocation},
    Event, IntoVal, Symbol,
};

#[test]
fn funding_moves_exactly_the_budget_into_escrow() {
    let s = Setup::new();
    let kernel = s.kernel();

    let id = s.funded_job(&kernel);

    assert_eq!(kernel.get_job(&id).state, JobState::Funded);
    assert_eq!(s.balance(&s.client), STARTING_BALANCE - BUDGET);
    assert_eq!(s.balance(&kernel.address), BUDGET);
}

#[test]
fn the_client_signs_for_exactly_one_transfer_of_the_budget_to_escrow() {
    // What a client's wallet is asked to approve. Anything more, or a
    // different amount or recipient, would be a way to take more than agreed.
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job(&kernel);
    kernel.set_budget(&id, &s.client, &BUDGET);

    kernel.fund(&id, &BUDGET);

    assert_eq!(
        s.env.auths(),
        std::vec![(
            s.client.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    kernel.address.clone(),
                    Symbol::new(&s.env, "fund"),
                    (id, BUDGET).into_val(&s.env),
                )),
                sub_invocations: std::vec![AuthorizedInvocation {
                    function: AuthorizedFunction::Contract((
                        s.token.address.clone(),
                        Symbol::new(&s.env, "transfer"),
                        (s.client.clone(), kernel.address.clone(), BUDGET).into_val(&s.env),
                    )),
                    sub_invocations: std::vec![],
                }],
            },
        )]
    );
}

#[test]
fn funding_emits_job_funded() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job(&kernel);
    kernel.set_budget(&id, &s.client, &BUDGET);

    kernel.fund(&id, &BUDGET);

    assert_eq!(
        s.events_of(&kernel.address),
        [JobFunded {
            id,
            client: s.client.clone(),
            amount: BUDGET
        }
        .to_xdr(&s.env, &kernel.address)]
    );
}

#[test]
fn funding_fails_if_the_budget_changed_since_the_client_agreed() {
    // T4: the provider raises the budget just before the client's funding
    // transaction lands.
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job(&kernel);
    kernel.set_budget(&id, &s.client, &BUDGET);
    kernel.set_budget(&id, &s.provider, &(BUDGET + USDC));

    assert_eq!(
        kernel.try_fund(&id, &BUDGET),
        Err(Ok(Error::BudgetMismatch))
    );
    assert_eq!(s.balance(&s.client), STARTING_BALANCE);
    assert_eq!(kernel.get_job(&id).state, JobState::Open);
}

#[test]
fn a_job_without_a_provider_cannot_be_funded() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job_without_provider(&kernel);
    kernel.set_budget(&id, &s.client, &BUDGET);

    assert_eq!(kernel.try_fund(&id, &BUDGET), Err(Ok(Error::NoProvider)));
}

#[test]
fn a_job_without_a_budget_cannot_be_funded() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job(&kernel);

    assert_eq!(kernel.try_fund(&id, &0), Err(Ok(Error::BadBudget)));
}

#[test]
fn a_job_is_funded_only_once() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.funded_job(&kernel);

    assert_eq!(kernel.try_fund(&id, &BUDGET), Err(Ok(Error::BadState)));
    assert_eq!(s.balance(&kernel.address), BUDGET);
}

#[test]
fn the_budget_cannot_change_after_funding() {
    // T5: the escrowed amount is what both sides agreed; neither can rewrite it.
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.funded_job(&kernel);

    for actor in [&s.client, &s.provider] {
        assert_eq!(
            kernel.try_set_budget(&id, actor, &USDC),
            Err(Ok(Error::BadState))
        );
    }
    assert_eq!(kernel.get_job(&id).budget, BUDGET);
}

#[test]
fn an_expired_job_cannot_be_funded() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job(&kernel);
    kernel.set_budget(&id, &s.client, &BUDGET);

    s.advance_time(JOB_DURATION);

    assert_eq!(kernel.try_fund(&id, &BUDGET), Err(Ok(Error::Expired)));
}

#[test]
fn funding_without_enough_tokens_changes_nothing() {
    // The transfer fails after the state was written; the whole call must
    // roll back, or the job would be Funded with an empty escrow.
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job(&kernel);
    let too_much = STARTING_BALANCE + 1;
    kernel.set_budget(&id, &s.client, &too_much);

    assert!(kernel.try_fund(&id, &too_much).is_err());

    assert_eq!(kernel.get_job(&id).state, JobState::Open);
    assert_eq!(s.balance(&s.client), STARTING_BALANCE);
    assert_eq!(s.balance(&kernel.address), 0);
}

#[test]
fn funding_an_unknown_job_fails() {
    let s = Setup::new();
    let kernel = s.kernel();

    assert_eq!(kernel.try_fund(&1, &BUDGET), Err(Ok(Error::NotFound)));
}
