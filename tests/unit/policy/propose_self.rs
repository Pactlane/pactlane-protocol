//! There is no separate cancel function. To withdraw a pending ownership
//! transfer, the owner proposes itself: the earlier candidate is replaced
//! and can no longer accept, and ownership never leaves the owner.

use pactlane_evaluation_policy::PolicyError;
use pactlane_tests::Setup;
use soroban_sdk::{testutils::Address as _, Address, IntoVal};

#[test]
fn proposing_itself_cancels_a_pending_transfer() {
    let s = Setup::new();
    let kernel = s.kernel();
    let owner = Address::generate(&s.env);
    let policy = s.policy(&kernel.address, &owner);
    let candidate = Address::generate(&s.env);
    policy.propose_owner(&candidate);

    policy.propose_owner(&owner);

    s.sign_only_as(
        &candidate,
        &policy.address,
        "accept_owner",
        ().into_val(&s.env),
        &[],
    );
    assert!(
        policy.try_accept_owner().is_err(),
        "the withdrawn candidate accepted"
    );
    s.env.mock_all_auths();
    assert_eq!(policy.owner(), owner);
}

#[test]
fn the_owner_can_clear_its_own_proposal_by_accepting_it() {
    // Accepting a self-proposal leaves the owner in place and nothing
    // pending, the same state as before any proposal was made.
    let s = Setup::new();
    let kernel = s.kernel();
    let owner = Address::generate(&s.env);
    let policy = s.policy(&kernel.address, &owner);
    policy.propose_owner(&Address::generate(&s.env));
    policy.propose_owner(&owner);

    policy.accept_owner();

    s.assert_authorized_by(&owner, &policy.address, "accept_owner");
    assert_eq!(policy.owner(), owner);
    assert_eq!(
        policy.try_accept_owner(),
        Err(Ok(PolicyError::NoPendingOwner))
    );
}
