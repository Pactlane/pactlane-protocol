use pactlane_commerce::DataKey;
use pactlane_interfaces::{Error, JobState, MAX_JOB_DURATION};
use pactlane_tests::{Setup, BUDGET, JOB_DURATION, STARTING_BALANCE};
use soroban_sdk::{
    testutils::storage::{Instance as _, Persistent as _},
    Address,
};

/// (job entry TTL, instance TTL, network maximum TTL), all in ledgers.
fn ttls(s: &Setup, kernel: &Address, id: u64) -> (u32, u32, u32) {
    s.env.as_contract(kernel, || {
        let storage = s.env.storage();
        (
            storage.persistent().get_ttl(&DataKey::Job(id)),
            storage.instance().get_ttl(),
            storage.max_ttl(),
        )
    })
}

#[test]
fn a_new_job_and_the_kernel_live_for_the_maximum_ttl() {
    let s = Setup::new();
    let kernel = s.kernel();

    let id = s.open_job(&kernel);

    let (job, instance, max) = ttls(&s, &kernel.address, id);
    assert_eq!(job, max);
    assert_eq!(instance, max);
}

#[test]
fn every_state_change_renews_the_maximum_ttl() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job(&kernel);
    kernel.set_budget(&id, &s.client, &BUDGET);

    s.advance_time(3 * 24 * 60 * 60);
    let (aged, _, max) = ttls(&s, &kernel.address, id);
    assert!(aged < max, "time must have eaten into the TTL");

    kernel.fund(&id, &BUDGET);

    let (job, instance, max) = ttls(&s, &kernel.address, id);
    assert_eq!(job, max);
    assert_eq!(instance, max);
}

#[test]
fn a_maximum_length_job_is_still_live_when_its_refund_opens() {
    // T12: funded on day one and never touched again, the job must still be
    // live on day 90 so the refund needs no restoration.
    //
    // A refund would succeed anyway: the host restores an archived entry
    // automatically when a call touches it (soroban-env-host 29,
    // `handle_maybe_expired_entry`). So asserting that the refund works would
    // pass with or without TTL management. The remaining TTL is what proves
    // the entry never archived.
    let s = Setup::new();
    let kernel = s.kernel();
    let id = kernel.create_job(
        &s.client,
        &Some(s.provider.clone()),
        &s.evaluator,
        &(s.now() + MAX_JOB_DURATION),
        &s.hash(1),
    );
    kernel.set_budget(&id, &s.client, &BUDGET);
    kernel.fund(&id, &BUDGET);
    let funded_at = s.env.ledger().sequence();

    s.advance_time(MAX_JOB_DURATION);

    let elapsed = s.env.ledger().sequence() - funded_at;
    let (job, instance, max) = ttls(&s, &kernel.address, id);
    assert_eq!(job, max - elapsed);
    assert_eq!(instance, max - elapsed);
    // Half the maximum TTL is still left: the 90-day cap has 2x headroom.
    assert!(job >= max / 2 - 1);

    kernel.claim_refund(&id);
    assert_eq!(kernel.get_job(&id).state, JobState::Expired);
    assert_eq!(s.balance(&s.client), STARTING_BALANCE);
}

#[test]
fn anyone_can_extend_a_job_without_a_signature() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job(&kernel);
    s.advance_time(JOB_DURATION);

    kernel.extend_ttl(&id);

    assert_eq!(s.env.auths(), std::vec![]);
    let (job, instance, max) = ttls(&s, &kernel.address, id);
    assert_eq!(job, max);
    assert_eq!(instance, max);
}

#[test]
fn extending_an_unknown_job_fails() {
    let s = Setup::new();
    let kernel = s.kernel();

    assert_eq!(kernel.try_extend_ttl(&1), Err(Ok(Error::NotFound)));
}
