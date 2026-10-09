//! Rotation, revocation, impersonation and replay (specs/EVALUATOR_TRUST.md).
//! The point of the policy is that changing who may act takes effect on
//! every job at once, including jobs funded before the change.

use pactlane_commerce::CommerceKernelClient;
use pactlane_evaluation_policy::{EvaluationPolicyClient, PolicyError};
use pactlane_interfaces::{Error, JobState};
use pactlane_tests::{Setup, BUDGET};
use soroban_sdk::{testutils::Address as _, Address, BytesN, IntoVal, InvokeError};

struct World {
    s: Setup,
    kernel: CommerceKernelClient<'static>,
    policy: EvaluationPolicyClient<'static>,
    owner: Address,
    signer: Address,
}

/// A policy with one signer, and a submitted job evaluated by the policy.
fn world() -> (World, u64) {
    let s = Setup::new();
    let kernel = s.kernel();
    let owner = Address::generate(&s.env);
    let policy = s.policy(&kernel.address, &owner);
    let signer = Address::generate(&s.env);
    policy.add_signer(&signer);
    let id = s.funded_job_evaluated_by(&kernel, &policy.address);
    kernel.submit(&id, &s.hash(2));
    (
        World {
            s,
            kernel,
            policy,
            owner,
            signer,
        },
        id,
    )
}

#[test]
fn a_removed_signer_is_locked_out_of_jobs_funded_before_its_removal() {
    // A compromised signer must lose power over every live job at once.
    let (w, id) = world();

    w.policy.remove_signer(&w.signer);

    assert_eq!(
        w.policy.try_complete(&w.signer, &id, &w.s.hash(9)),
        Err(Ok(PolicyError::NotSigner))
    );
    assert_eq!(
        w.policy.try_reject(&w.signer, &id, &None),
        Err(Ok(PolicyError::NotSigner))
    );
    assert_eq!(w.kernel.get_job(&id).state, JobState::Submitted);
}

#[test]
fn a_signer_added_after_funding_can_settle_the_job() {
    // The replacement for a removed signer must be able to finish its work.
    let (w, id) = world();
    w.policy.remove_signer(&w.signer);
    let replacement = Address::generate(&w.s.env);
    w.policy.add_signer(&replacement);

    w.policy.complete(&replacement, &id, &w.s.hash(9));

    assert_eq!(w.kernel.get_job(&id).state, JobState::Completed);
    assert_eq!(w.s.balance(&w.s.provider), BUDGET);
}

#[test]
fn a_new_owner_takes_over_the_signer_set_for_existing_jobs() {
    let (w, id) = world();
    let next = Address::generate(&w.s.env);
    w.policy.propose_owner(&next);
    w.policy.accept_owner();

    // The old owner can no longer change signers...
    w.s.sign_only_as(
        &w.owner,
        &w.policy.address,
        "remove_signer",
        (w.signer.clone(),).into_val(&w.s.env),
        &[],
    );
    assert_eq!(
        w.policy.try_remove_signer(&w.signer),
        Err(Err(InvokeError::Abort))
    );
    assert!(w.policy.is_signer(&w.signer));

    // ...and the new one can, with effect on the job funded earlier.
    w.s.sign_only_as(
        &next,
        &w.policy.address,
        "remove_signer",
        (w.signer.clone(),).into_val(&w.s.env),
        &[],
    );
    w.policy.remove_signer(&w.signer);
    // Back to mocked signatures, so membership, not authorization, decides.
    w.s.env.mock_all_auths();
    assert_eq!(
        w.policy.try_complete(&w.signer, &id, &w.s.hash(9)),
        Err(Ok(PolicyError::NotSigner))
    );
}

#[test]
fn nobody_can_settle_in_a_signers_name() {
    // The signer is an argument, so its own signature must be what
    // authorizes the call; anyone else signing must be refused.
    for impostor in ["stranger", "owner", "provider"] {
        let (w, id) = world();
        let impostor = match impostor {
            "stranger" => w.s.stranger.clone(),
            "owner" => w.owner.clone(),
            _ => w.s.provider.clone(),
        };
        w.s.sign_only_as(
            &impostor,
            &w.policy.address,
            "complete",
            (w.signer.clone(), id, w.s.hash(9)).into_val(&w.s.env),
            &[],
        );

        assert_eq!(
            w.policy.try_complete(&w.signer, &id, &w.s.hash(9)),
            Err(Err(InvokeError::Abort))
        );
        assert_eq!(w.kernel.get_job(&id).state, JobState::Submitted);
    }
}

#[test]
fn only_the_owner_manages_signers() {
    for (function, args) in [("add_signer", true), ("remove_signer", false)] {
        let (w, _) = world();
        let target = if args {
            Address::generate(&w.s.env)
        } else {
            w.signer.clone()
        };
        let before = w.policy.is_signer(&target);
        w.s.sign_only_as(
            &w.signer,
            &w.policy.address,
            function,
            (target.clone(),).into_val(&w.s.env),
            &[],
        );

        let result = if args {
            w.policy.try_add_signer(&target)
        } else {
            w.policy.try_remove_signer(&target)
        };

        assert_eq!(
            result,
            Err(Err(InvokeError::Abort)),
            "{function} by a signer"
        );
        assert_eq!(w.policy.is_signer(&target), before);
    }
}

#[test]
fn only_the_proposed_owner_can_accept() {
    let (w, _) = world();
    let next = Address::generate(&w.s.env);
    w.policy.propose_owner(&next);
    w.s.sign_only_as(
        &w.s.stranger,
        &w.policy.address,
        "accept_owner",
        ().into_val(&w.s.env),
        &[],
    );

    assert_eq!(w.policy.try_accept_owner(), Err(Err(InvokeError::Abort)));
    assert_eq!(w.policy.owner(), w.owner);
}

#[test]
fn a_signed_verdict_cannot_be_moved_to_another_job() {
    // The job ID is part of the signed arguments, so a signature approving
    // job A grants nothing on job B.
    let (w, a) = world();
    let b = w.s.funded_job_evaluated_by(&w.kernel, &w.policy.address);
    w.kernel.submit(&b, &w.s.hash(2));
    w.s.sign_only_as(
        &w.signer,
        &w.policy.address,
        "complete",
        (w.signer.clone(), a, w.s.hash(9)).into_val(&w.s.env),
        &[],
    );

    assert_eq!(
        w.policy.try_complete(&w.signer, &b, &w.s.hash(9)),
        Err(Err(InvokeError::Abort))
    );
    assert_eq!(w.kernel.get_job(&b).state, JobState::Submitted);
}

#[test]
fn a_settled_job_cannot_be_settled_again_through_the_policy() {
    // T6 via the policy: the kernel's terminal state still holds.
    let (w, id) = world();
    w.policy.complete(&w.signer, &id, &w.s.hash(9));

    assert_eq!(
        w.policy.try_complete(&w.signer, &id, &w.s.hash(9)),
        Err(Err(InvokeError::Contract(Error::BadState as u32)))
    );
    assert_eq!(
        w.policy
            .try_reject(&w.signer, &id, &Option::<BytesN<32>>::None),
        Err(Err(InvokeError::Contract(Error::BadState as u32)))
    );
    assert_eq!(w.s.balance(&w.s.provider), BUDGET);
}
