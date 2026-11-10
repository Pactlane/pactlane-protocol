//! A proposed owner has no power until it accepts. Signed only by the
//! pending owner, every owner action fails and the current owner keeps
//! control.

use pactlane_tests::Setup;
use soroban_sdk::{testutils::Address as _, Address, IntoVal};

#[test]
fn a_pending_owner_cannot_manage_signers_or_ownership_before_accepting() {
    let s = Setup::new();
    let kernel = s.kernel();
    let owner = Address::generate(&s.env);
    let policy = s.policy(&kernel.address, &owner);
    let existing = Address::generate(&s.env);
    policy.add_signer(&existing);
    let pending = Address::generate(&s.env);
    policy.propose_owner(&pending);
    let newcomer = Address::generate(&s.env);
    let p = &policy.address;

    s.sign_only_as(
        &pending,
        p,
        "add_signer",
        (newcomer.clone(),).into_val(&s.env),
        &[],
    );
    assert!(policy.try_add_signer(&newcomer).is_err());
    s.sign_only_as(
        &pending,
        p,
        "remove_signer",
        (existing.clone(),).into_val(&s.env),
        &[],
    );
    assert!(policy.try_remove_signer(&existing).is_err());
    s.sign_only_as(
        &pending,
        p,
        "propose_owner",
        (newcomer.clone(),).into_val(&s.env),
        &[],
    );
    assert!(policy.try_propose_owner(&newcomer).is_err());
    s.env.mock_all_auths();

    assert!(!policy.is_signer(&newcomer));
    assert!(policy.is_signer(&existing));
    assert_eq!(policy.owner(), owner);
    // The proposal itself is untouched: the pending owner can still accept.
    policy.accept_owner();
    assert_eq!(policy.owner(), pending);
}
