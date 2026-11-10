//! A refused call must publish no kernel event.
//!
//! `events_of` alone cannot show that: the SDK drops events from failed
//! calls before returning them, so it is empty after any refusal whatever the
//! kernel did. The host's raw buffer keeps those events, flagged
//! `failed_call`, so these tests read it too. That catches a kernel that
//! publishes an event and then refuses, which an indexer replaying
//! diagnostic output, or a future SDK, could surface as a real event.

use core::fmt::Debug;

use pactlane_interfaces::{Error, JobExpired};
use pactlane_tests::{Setup, BUDGET, JOB_DURATION, STARTING_BALANCE};
use soroban_sdk::{
    contract, contractimpl,
    xdr::{ContractEventType, ScAddress},
    Address, Env,
};

/// Every contract event `contract` published during the last call, including
/// those discarded because the call failed.
fn published_by(s: &Setup, contract: &Address) -> std::vec::Vec<soroban_sdk::xdr::ContractEvent> {
    let ScAddress::Contract(id) = ScAddress::from(contract) else {
        panic!("{contract:?} is not a contract");
    };
    s.env
        .host()
        .get_events()
        .unwrap()
        .0
        .into_iter()
        .filter(|e| e.event.type_ == ContractEventType::Contract)
        .filter(|e| e.event.contract_id.as_ref() == Some(&id))
        .map(|e| e.event)
        .collect()
}

fn assert_published_nothing(s: &Setup, kernel: &Address, call: &str) {
    assert!(
        s.events_of(kernel).events().is_empty(),
        "{call}: emitted kernel events"
    );
    let published = published_by(s, kernel);
    assert!(
        published.is_empty(),
        "{call}: published {published:?} before refusing"
    );
}

/// Asserts a call was refused with `error` and published nothing. `outcome`
/// must be the result of the call just made, before anything else runs.
fn assert_refused_silently<T: Debug, E: Debug>(
    s: &Setup,
    kernel: &Address,
    call: &str,
    outcome: Result<T, Result<Error, E>>,
    error: Error,
) {
    match outcome {
        Err(Ok(refused)) => assert_eq!(refused, error, "{call}"),
        other => panic!("{call}: expected {error:?}, got {other:?}"),
    }
    assert_published_nothing(s, kernel, call);
}

#[test]
fn every_refused_kernel_call_publishes_no_event() {
    let s = Setup::new();
    let kernel = s.kernel();
    let k = &kernel.address;
    let open = s.open_job(&kernel);
    kernel.set_budget(&open, &s.client, &BUDGET);
    let bare = s.open_job_without_provider(&kernel);
    kernel.set_budget(&bare, &s.client, &BUDGET);
    let funded = s.funded_job(&kernel);
    let paid = s.submitted_job(&kernel);
    kernel.complete(&paid, &s.hash(9));
    let later = s.now() + JOB_DURATION;

    assert_refused_silently(
        &s,
        k,
        "create_job expiring now",
        kernel.try_create_job(&s.client, &None, &s.evaluator, &s.now(), &s.hash(1)),
        Error::BadExpiry,
    );
    assert_refused_silently(
        &s,
        k,
        "create_job evaluated by its client",
        kernel.try_create_job(&s.client, &None, &s.client, &later, &s.hash(1)),
        Error::RoleConflict,
    );
    assert_refused_silently(
        &s,
        k,
        "set_provider on a job that has one",
        kernel.try_set_provider(&open, &s.stranger),
        Error::ProviderAlreadySet,
    );
    assert_refused_silently(
        &s,
        k,
        "set_provider to the evaluator",
        kernel.try_set_provider(&bare, &s.evaluator),
        Error::RoleConflict,
    );
    assert_refused_silently(
        &s,
        k,
        "set_budget by a stranger",
        kernel.try_set_budget(&open, &s.stranger, &BUDGET),
        Error::BadActor,
    );
    assert_refused_silently(
        &s,
        k,
        "set_budget of zero",
        kernel.try_set_budget(&open, &s.client, &0),
        Error::BadBudget,
    );
    assert_refused_silently(
        &s,
        k,
        "fund with a different budget",
        kernel.try_fund(&open, &(BUDGET + 1)),
        Error::BudgetMismatch,
    );
    assert_refused_silently(
        &s,
        k,
        "fund without a provider",
        kernel.try_fund(&bare, &BUDGET),
        Error::NoProvider,
    );
    assert_refused_silently(
        &s,
        k,
        "submit to an open job",
        kernel.try_submit(&open, &s.hash(2)),
        Error::BadState,
    );
    assert_refused_silently(
        &s,
        k,
        "complete unsubmitted work",
        kernel.try_complete(&funded, &s.hash(9)),
        Error::BadState,
    );
    assert_refused_silently(
        &s,
        k,
        "reject a completed job",
        kernel.try_reject(&paid, &None),
        Error::BadState,
    );
    assert_refused_silently(
        &s,
        k,
        "claim_refund on a live job",
        kernel.try_claim_refund(&funded),
        Error::NotExpired,
    );
    assert_refused_silently(
        &s,
        k,
        "extend_ttl on an unknown job",
        kernel.try_extend_ttl(&99),
        Error::NotFound,
    );
    assert_refused_silently(
        &s,
        k,
        "get_job on an unknown job",
        kernel.try_get_job(&99),
        Error::NotFound,
    );
}

#[test]
fn calls_refused_for_expiry_publish_no_event() {
    let s = Setup::new();
    let kernel = s.kernel();
    let k = &kernel.address;
    let open = s.open_job(&kernel);
    kernel.set_budget(&open, &s.client, &BUDGET);
    let bare = s.open_job_without_provider(&kernel);
    let funded = s.funded_job(&kernel);
    let submitted = s.submitted_job(&kernel);
    s.advance_time(JOB_DURATION);

    assert_refused_silently(
        &s,
        k,
        "set_provider after expiry",
        kernel.try_set_provider(&bare, &s.provider),
        Error::Expired,
    );
    assert_refused_silently(
        &s,
        k,
        "set_budget after expiry",
        kernel.try_set_budget(&open, &s.client, &BUDGET),
        Error::Expired,
    );
    assert_refused_silently(
        &s,
        k,
        "fund after expiry",
        kernel.try_fund(&open, &BUDGET),
        Error::Expired,
    );
    assert_refused_silently(
        &s,
        k,
        "submit after expiry",
        kernel.try_submit(&funded, &s.hash(2)),
        Error::Expired,
    );
    assert_refused_silently(
        &s,
        k,
        "complete after expiry",
        kernel.try_complete(&submitted, &s.hash(9)),
        Error::Expired,
    );
    assert_refused_silently(
        &s,
        k,
        "evaluator reject after expiry",
        kernel.try_reject(&funded, &None),
        Error::Expired,
    );
}

#[test]
fn a_funding_whose_transfer_fails_publishes_no_event() {
    // The one refusal that comes after the kernel has already written the
    // job's new state: the token refuses the transfer, and the job_funded
    // event that would follow it must not have been published either.
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job(&kernel);
    let too_much = STARTING_BALANCE + 1;
    kernel.set_budget(&id, &s.client, &too_much);

    assert!(kernel.try_fund(&id, &too_much).is_err());

    assert_published_nothing(&s, &kernel.address, "fund beyond the client's balance");
}

/// Publishes a kernel event and then refuses, the bug the checks above are
/// written to catch.
#[contract]
struct PublishesThenRefuses;

#[contractimpl]
impl PublishesThenRefuses {
    pub fn run(env: Env) -> Result<(), Error> {
        soroban_sdk::Event::publish(&JobExpired { id: 1 }, &env);
        Err(Error::BadState)
    }
}

#[test]
fn an_event_published_before_a_refusal_is_still_detected() {
    // Proves the raw-buffer check is not vacuous: the SDK's filtered view
    // hides this event, but the check sees it.
    let s = Setup::new();
    let buggy = s.env.register(PublishesThenRefuses, ());
    let client = PublishesThenRefusesClient::new(&s.env, &buggy);

    assert_eq!(client.try_run(), Err(Ok(Error::BadState)));

    assert!(s.events_of(&buggy).events().is_empty());
    assert_eq!(published_by(&s, &buggy).len(), 1);
}
