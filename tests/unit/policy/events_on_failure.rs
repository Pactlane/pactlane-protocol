//! A refused policy call publishes no event, from the policy or from the
//! kernel it calls. As in the kernel's version of this test, the host's raw
//! event buffer is read, because the SDK's filtered view drops events from
//! failed calls and would pass whatever the contract did.

use core::fmt::Debug;

use pactlane_evaluation_policy::PolicyError;
use pactlane_tests::Setup;
use soroban_sdk::{
    testutils::Address as _,
    xdr::{ContractEventType, ScAddress},
    Address,
};

fn published_by(s: &Setup, contract: &Address) -> usize {
    let ScAddress::Contract(id) = ScAddress::from(contract) else {
        panic!("{contract:?} is not a contract");
    };
    s.env
        .host()
        .get_events()
        .unwrap()
        .0
        .iter()
        .filter(|e| e.event.type_ == ContractEventType::Contract)
        .filter(|e| e.event.contract_id.as_ref() == Some(&id))
        .count()
}

fn assert_silent<T: Debug, E: Debug>(
    s: &Setup,
    contracts: [&Address; 2],
    call: &str,
    outcome: Result<T, E>,
) {
    assert!(
        outcome.is_err(),
        "{call} should have been refused: {outcome:?}"
    );
    for contract in contracts {
        assert!(s.events_of(contract).events().is_empty(), "{call}");
        assert_eq!(
            published_by(s, contract),
            0,
            "{call} published before refusing"
        );
    }
}

#[test]
fn every_refused_policy_call_publishes_no_event() {
    let s = Setup::new();
    let kernel = s.kernel();
    let policy = s.policy(&kernel.address, &Address::generate(&s.env));
    let signer = Address::generate(&s.env);
    policy.add_signer(&signer);
    let funded = s.funded_job_evaluated_by(&kernel, &policy.address);
    let outsider = Address::generate(&s.env);
    let both = [&policy.address, &kernel.address];

    let outcome = policy.try_add_signer(&signer);
    assert_eq!(outcome, Err(Ok(PolicyError::AlreadySigner)));
    assert_silent(&s, both, "add_signer twice", outcome);
    let outcome = policy.try_remove_signer(&outsider);
    assert_eq!(outcome, Err(Ok(PolicyError::NotSigner)));
    assert_silent(&s, both, "remove_signer of a non-signer", outcome);
    let outcome = policy.try_accept_owner();
    assert_eq!(outcome, Err(Ok(PolicyError::NoPendingOwner)));
    assert_silent(&s, both, "accept_owner with nothing pending", outcome);
    let outcome = policy.try_complete(&outsider, &funded, &s.hash(9));
    assert_eq!(outcome, Err(Ok(PolicyError::NotSigner)));
    assert_silent(&s, both, "complete by a non-signer", outcome);

    // A signer who is the job's client.
    policy.add_signer(&s.client);
    let outcome = policy.try_reject(&s.client, &funded, &None);
    assert_eq!(outcome, Err(Ok(PolicyError::SignerConflict)));
    assert_silent(&s, both, "reject by a party to the job", outcome);

    // The kernel refuses: completing work that was never submitted.
    assert_silent(
        &s,
        both,
        "complete before submission",
        policy.try_complete(&signer, &funded, &s.hash(9)),
    );
    assert_silent(
        &s,
        both,
        "settle an unknown job",
        policy.try_reject(&signer, &99, &None),
    );
}
