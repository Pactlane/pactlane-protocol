use pactlane_evaluation_policy::{
    EvaluationPolicy, OwnerChanged, OwnerProposed, PolicyError, SignerAdded, SignerRemoved,
};
use pactlane_tests::Setup;
use soroban_sdk::{testutils::Address as _, Address, Event};

#[test]
fn a_policy_is_bound_to_its_owner_and_kernel() {
    let s = Setup::new();
    let kernel = s.kernel();
    let owner = Address::generate(&s.env);

    let policy = s.policy(&kernel.address, &owner);

    assert_eq!(policy.owner(), owner);
    assert_eq!(policy.kernel(), kernel.address);
}

#[test]
#[should_panic(expected = "\"contract call failed\", job_count")]
fn binding_a_policy_to_an_address_that_is_not_a_kernel_fails() {
    // Like T11 for the kernel: a policy bound to a non-kernel could never
    // settle anything.
    let s = Setup::new();
    let not_a_kernel = Address::generate(&s.env);
    s.env
        .register(EvaluationPolicy, (Address::generate(&s.env), not_a_kernel));
}

#[test]
fn only_the_owner_adds_a_signer() {
    let s = Setup::new();
    let kernel = s.kernel();
    let owner = Address::generate(&s.env);
    let policy = s.policy(&kernel.address, &owner);
    let signer = Address::generate(&s.env);

    policy.add_signer(&signer);

    s.assert_authorized_by(&owner, &policy.address, "add_signer");
    assert_eq!(
        s.events_of(&policy.address),
        [SignerAdded {
            signer: signer.clone()
        }
        .to_xdr(&s.env, &policy.address)]
    );
    assert!(policy.is_signer(&signer));
}

#[test]
fn a_signer_is_added_only_once() {
    let s = Setup::new();
    let kernel = s.kernel();
    let policy = s.policy(&kernel.address, &Address::generate(&s.env));
    let signer = Address::generate(&s.env);
    policy.add_signer(&signer);

    assert_eq!(
        policy.try_add_signer(&signer),
        Err(Ok(PolicyError::AlreadySigner))
    );
}

#[test]
fn only_the_owner_removes_a_signer() {
    let s = Setup::new();
    let kernel = s.kernel();
    let owner = Address::generate(&s.env);
    let policy = s.policy(&kernel.address, &owner);
    let signer = Address::generate(&s.env);
    policy.add_signer(&signer);

    policy.remove_signer(&signer);

    s.assert_authorized_by(&owner, &policy.address, "remove_signer");
    assert_eq!(
        s.events_of(&policy.address),
        [SignerRemoved {
            signer: signer.clone()
        }
        .to_xdr(&s.env, &policy.address)]
    );
    assert!(!policy.is_signer(&signer));
}

#[test]
fn removing_an_address_that_is_not_a_signer_fails() {
    let s = Setup::new();
    let kernel = s.kernel();
    let policy = s.policy(&kernel.address, &Address::generate(&s.env));

    assert_eq!(
        policy.try_remove_signer(&Address::generate(&s.env)),
        Err(Ok(PolicyError::NotSigner))
    );
}

#[test]
fn ownership_moves_only_when_the_new_owner_accepts() {
    // A transfer to a mistyped address must not lose control of the policy.
    let s = Setup::new();
    let kernel = s.kernel();
    let owner = Address::generate(&s.env);
    let next = Address::generate(&s.env);
    let policy = s.policy(&kernel.address, &owner);

    policy.propose_owner(&next);

    s.assert_authorized_by(&owner, &policy.address, "propose_owner");
    assert_eq!(
        s.events_of(&policy.address),
        [OwnerProposed {
            owner: owner.clone(),
            pending: next.clone()
        }
        .to_xdr(&s.env, &policy.address)]
    );
    assert_eq!(policy.owner(), owner);

    policy.accept_owner();

    s.assert_authorized_by(&next, &policy.address, "accept_owner");
    assert_eq!(
        s.events_of(&policy.address),
        [OwnerChanged {
            old_owner: owner.clone(),
            new_owner: next.clone()
        }
        .to_xdr(&s.env, &policy.address)]
    );
    assert_eq!(policy.owner(), next);

    // From now on the new owner, not the old one, manages signers.
    policy.add_signer(&Address::generate(&s.env));
    s.assert_authorized_by(&next, &policy.address, "add_signer");
}

#[test]
fn accepting_with_no_proposal_outstanding_fails() {
    let s = Setup::new();
    let kernel = s.kernel();
    let policy = s.policy(&kernel.address, &Address::generate(&s.env));

    assert_eq!(
        policy.try_accept_owner(),
        Err(Ok(PolicyError::NoPendingOwner))
    );
}

#[test]
fn a_proposal_is_accepted_only_once() {
    let s = Setup::new();
    let kernel = s.kernel();
    let policy = s.policy(&kernel.address, &Address::generate(&s.env));
    policy.propose_owner(&Address::generate(&s.env));
    policy.accept_owner();

    assert_eq!(
        policy.try_accept_owner(),
        Err(Ok(PolicyError::NoPendingOwner))
    );
}
