use pactlane_evaluation_policy::{EvaluationPolicyClient, PolicyError, Settled};
use pactlane_interfaces::{Error, JobCompleted, JobState, PaymentReleased};
use pactlane_tests::{Setup, BUDGET, JOB_DURATION, STARTING_BALANCE};
use soroban_sdk::{testutils::Address as _, Address, ConversionError, Event, InvokeError};

type Outcome = Result<Result<(), ConversionError>, Result<PolicyError, InvokeError>>;

/// How a kernel error reaches a caller of the policy: the same code, unchanged.
fn kernel_error(error: Error) -> Outcome {
    Err(Err(InvokeError::Contract(error as u32)))
}

/// A kernel, a policy with one signer, and a job evaluated by the policy.
fn policy_with_signer(
    s: &Setup,
) -> (
    pactlane_commerce::CommerceKernelClient<'static>,
    EvaluationPolicyClient<'static>,
    Address,
) {
    let kernel = s.kernel();
    let policy = s.policy(&kernel.address, &Address::generate(&s.env));
    let signer = Address::generate(&s.env);
    policy.add_signer(&signer);
    (kernel, policy, signer)
}

#[test]
fn a_signer_completes_a_job_through_the_policy() {
    let s = Setup::new();
    let (kernel, policy, signer) = policy_with_signer(&s);
    let id = s.funded_job_evaluated_by(&kernel, &policy.address);
    kernel.submit(&id, &s.hash(2));

    policy.complete(&signer, &id, &s.hash(9));

    // Only the signer signs; the kernel accepts the policy as the direct
    // caller, without a separate signature.
    s.assert_authorized_by(&signer, &policy.address, "complete");
    assert_eq!(
        s.events_of(&kernel.address),
        [
            JobCompleted {
                id,
                evaluator: policy.address.clone(),
                reason: s.hash(9)
            }
            .to_xdr(&s.env, &kernel.address),
            PaymentReleased {
                id,
                provider: s.provider.clone(),
                amount: BUDGET
            }
            .to_xdr(&s.env, &kernel.address),
        ]
    );
    // The kernel only knows the policy; this names who actually decided.
    assert_eq!(
        s.events_of(&policy.address),
        [Settled {
            id,
            signer: signer.clone(),
            completed: true,
            reason: Some(s.hash(9))
        }
        .to_xdr(&s.env, &policy.address)]
    );
    assert_eq!(s.balance(&s.provider), BUDGET);
}

#[test]
fn a_signer_rejects_a_job_through_the_policy() {
    let s = Setup::new();
    let (kernel, policy, signer) = policy_with_signer(&s);
    let id = s.funded_job_evaluated_by(&kernel, &policy.address);

    policy.reject(&signer, &id, &Some(s.hash(8)));

    s.assert_authorized_by(&signer, &policy.address, "reject");
    assert_eq!(
        s.events_of(&policy.address),
        [Settled {
            id,
            signer: signer.clone(),
            completed: false,
            reason: Some(s.hash(8))
        }
        .to_xdr(&s.env, &policy.address)]
    );
    assert_eq!(kernel.get_job(&id).state, JobState::Rejected);
    assert_eq!(s.balance(&s.client), STARTING_BALANCE);
}

#[test]
fn an_address_outside_the_signer_set_cannot_settle() {
    let s = Setup::new();
    let (kernel, policy, _) = policy_with_signer(&s);
    let id = s.funded_job_evaluated_by(&kernel, &policy.address);
    kernel.submit(&id, &s.hash(2));

    for outsider in [&s.stranger, &policy.owner()] {
        assert_eq!(
            policy.try_complete(outsider, &id, &s.hash(9)),
            Err(Ok(PolicyError::NotSigner))
        );
        assert_eq!(
            policy.try_reject(outsider, &id, &None),
            Err(Ok(PolicyError::NotSigner))
        );
    }
    assert_eq!(kernel.get_job(&id).state, JobState::Submitted);
}

#[test]
fn a_signer_cannot_settle_a_job_it_is_a_party_to() {
    // T1 through indirection: the kernel sees only the policy, so the policy
    // must refuse a signer who is the job's provider (approving its own work)
    // or client (rejecting work to get its money back).
    let s = Setup::new();
    let (kernel, policy, _) = policy_with_signer(&s);
    policy.add_signer(&s.provider);
    policy.add_signer(&s.client);
    let id = s.funded_job_evaluated_by(&kernel, &policy.address);
    kernel.submit(&id, &s.hash(2));

    for party in [&s.provider, &s.client] {
        assert_eq!(
            policy.try_complete(party, &id, &s.hash(9)),
            Err(Ok(PolicyError::SignerConflict))
        );
        assert_eq!(
            policy.try_reject(party, &id, &None),
            Err(Ok(PolicyError::SignerConflict))
        );
    }
    assert_eq!(kernel.get_job(&id).state, JobState::Submitted);
    assert_eq!(s.balance(&s.provider), 0);
}

#[test]
fn the_kernels_rules_still_apply_through_the_policy() {
    // The policy adds checks; it never removes the kernel's. Kernel errors
    // pass through unchanged.
    let s = Setup::new();
    let (kernel, policy, signer) = policy_with_signer(&s);
    let funded = s.funded_job_evaluated_by(&kernel, &policy.address);
    let submitted = s.funded_job_evaluated_by(&kernel, &policy.address);
    kernel.submit(&submitted, &s.hash(2));

    // Completing work that was never submitted.
    assert_eq!(
        policy.try_complete(&signer, &funded, &s.hash(9)),
        kernel_error(Error::BadState)
    );
    // Settling after expiry.
    s.advance_time(JOB_DURATION);
    assert_eq!(
        policy.try_complete(&signer, &submitted, &s.hash(9)),
        kernel_error(Error::Expired)
    );
    assert_eq!(
        policy.try_reject(&signer, &submitted, &None),
        kernel_error(Error::Expired)
    );

    assert_eq!(kernel.get_job(&funded).state, JobState::Funded);
    assert_eq!(kernel.get_job(&submitted).state, JobState::Submitted);
    assert_eq!(s.balance(&s.provider), 0);
}

#[test]
fn a_policy_cannot_settle_a_job_with_a_different_evaluator() {
    // Being a signer somewhere grants nothing over jobs that named someone
    // else as evaluator.
    let s = Setup::new();
    let (kernel, policy, signer) = policy_with_signer(&s);
    let id = s.submitted_job(&kernel); // evaluated by s.evaluator

    // The kernel asks for the job's real evaluator, which never signed.
    assert_eq!(
        policy.try_complete(&signer, &id, &s.hash(9)),
        Err(Err(InvokeError::Abort))
    );

    assert_eq!(kernel.get_job(&id).state, JobState::Submitted);
    assert_eq!(s.balance(&s.provider), 0);
}

#[test]
fn settling_an_unknown_job_fails() {
    let s = Setup::new();
    let (_, policy, signer) = policy_with_signer(&s);

    assert_eq!(
        policy.try_complete(&signer, &99, &s.hash(9)),
        kernel_error(Error::NotFound)
    );
}
