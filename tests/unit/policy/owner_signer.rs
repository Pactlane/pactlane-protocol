//! Owner and signer are separate powers, and one address may hold both. The
//! settlement is still attributed to it as the signer.

use pactlane_evaluation_policy::Settled;
use pactlane_interfaces::JobState;
use pactlane_tests::Setup;
use soroban_sdk::{testutils::Address as _, Address, Event};

#[test]
fn the_owner_can_be_a_signer_and_settles_in_its_own_name() {
    let s = Setup::new();
    let kernel = s.kernel();
    let owner = Address::generate(&s.env);
    let policy = s.policy(&kernel.address, &owner);
    policy.add_signer(&owner);
    let id = s.funded_job_evaluated_by(&kernel, &policy.address);
    kernel.submit(&id, &s.hash(2));

    policy.complete(&owner, &id, &s.hash(9));

    s.assert_authorized_by(&owner, &policy.address, "complete");
    let settled = Settled {
        id,
        signer: owner.clone(),
        completed: true,
        reason: Some(s.hash(9)),
    }
    .to_xdr(&s.env, &policy.address);
    assert_eq!(s.events_of(&policy.address), [settled]);
    assert_eq!(kernel.get_job(&id).state, JobState::Completed);
    assert!(policy.is_signer(&owner));
    assert_eq!(policy.owner(), owner);
}
