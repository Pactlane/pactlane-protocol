//! A policy is a shared evaluator: one signer settles jobs for any number
//! of clients, and each settlement moves only its own job's money.

use pactlane_interfaces::JobState;
use pactlane_tests::{Setup, BUDGET, JOB_DURATION, STARTING_BALANCE};
use soroban_sdk::{testutils::Address as _, Address};

#[test]
fn one_signer_settles_jobs_from_three_clients() {
    let s = Setup::new();
    let kernel = s.kernel();
    let policy = s.policy(&kernel.address, &Address::generate(&s.env));
    let signer = Address::generate(&s.env);
    policy.add_signer(&signer);
    let clients = [
        s.client.clone(),
        Address::generate(&s.env),
        Address::generate(&s.env),
    ];
    for client in &clients[1..] {
        s.token_admin.mint(client, &STARTING_BALANCE);
    }

    let jobs: std::vec::Vec<u64> = clients
        .iter()
        .map(|client| {
            let id = kernel.create_job(
                client,
                &Some(s.provider.clone()),
                &policy.address,
                &(s.now() + JOB_DURATION),
                &s.hash(1),
            );
            kernel.set_budget(&id, client, &BUDGET);
            kernel.fund(&id, &BUDGET);
            id
        })
        .collect();
    kernel.submit(&jobs[0], &s.hash(2));
    kernel.submit(&jobs[2], &s.hash(2));

    policy.complete(&signer, &jobs[0], &s.hash(9));
    policy.reject(&signer, &jobs[1], &None);
    policy.complete(&signer, &jobs[2], &s.hash(9));

    for (id, state) in
        jobs.iter()
            .zip([JobState::Completed, JobState::Rejected, JobState::Completed])
    {
        assert_eq!(kernel.get_job(id).state, state, "job {id}");
    }
    assert_eq!(s.balance(&clients[0]), STARTING_BALANCE - BUDGET);
    assert_eq!(s.balance(&clients[1]), STARTING_BALANCE);
    assert_eq!(s.balance(&clients[2]), STARTING_BALANCE - BUDGET);
    assert_eq!(s.balance(&s.provider), 2 * BUDGET);
    assert_eq!(s.balance(&kernel.address), 0);
}
