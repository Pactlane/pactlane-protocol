//! A second ownership proposal replaces the first: only the latest
//! candidate can accept, so a mistaken proposal can be corrected.

use pactlane_evaluation_policy::PolicyError;
use pactlane_tests::Setup;
use soroban_sdk::{testutils::Address as _, Address, IntoVal};

#[test]
fn only_the_latest_proposed_owner_can_accept() {
    let s = Setup::new();
    let kernel = s.kernel();
    let owner = Address::generate(&s.env);
    let policy = s.policy(&kernel.address, &owner);
    let first = Address::generate(&s.env);
    let second = Address::generate(&s.env);

    policy.propose_owner(&first);
    policy.propose_owner(&second);

    // The superseded candidate signs, and is refused.
    s.sign_only_as(
        &first,
        &policy.address,
        "accept_owner",
        ().into_val(&s.env),
        &[],
    );
    assert!(policy.try_accept_owner().is_err());
    s.env.mock_all_auths();
    assert_eq!(policy.owner(), owner);

    policy.accept_owner();

    s.assert_authorized_by(&second, &policy.address, "accept_owner");
    assert_eq!(policy.owner(), second);
    assert_eq!(
        policy.try_accept_owner(),
        Err(Ok(PolicyError::NoPendingOwner))
    );
}
