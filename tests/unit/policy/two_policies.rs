//! Each policy speaks only for jobs that name it as evaluator. A signer of
//! policy A gains no power over jobs evaluated by policy B on the same
//! kernel, whichever policy it calls.

use pactlane_evaluation_policy::PolicyError;
use pactlane_interfaces::JobState;
use pactlane_tests::{Setup, BUDGET};
use soroban_sdk::{testutils::Address as _, Address};

#[test]
fn a_signer_of_one_policy_cannot_settle_jobs_evaluated_by_another() {
    let s = Setup::new();
    let kernel = s.kernel();
    let a = s.policy(&kernel.address, &Address::generate(&s.env));
    let b = s.policy(&kernel.address, &Address::generate(&s.env));
    let signer_a = Address::generate(&s.env);
    let signer_b = Address::generate(&s.env);
    a.add_signer(&signer_a);
    b.add_signer(&signer_b);
    let job_b = s.funded_job_evaluated_by(&kernel, &b.address);
    kernel.submit(&job_b, &s.hash(2));
    let before = kernel.get_job(&job_b);

    // Through B: not one of B's signers.
    assert_eq!(
        b.try_complete(&signer_a, &job_b, &s.hash(9)),
        Err(Ok(PolicyError::NotSigner))
    );
    // Through A: A is not this job's evaluator, so the kernel refuses A.
    assert!(a.try_complete(&signer_a, &job_b, &s.hash(9)).is_err());
    assert!(a.try_reject(&signer_a, &job_b, &None).is_err());

    assert_eq!(kernel.get_job(&job_b), before);
    assert_eq!(s.balance(&kernel.address), BUDGET);

    b.complete(&signer_b, &job_b, &s.hash(9));
    assert_eq!(kernel.get_job(&job_b).state, JobState::Completed);
}
