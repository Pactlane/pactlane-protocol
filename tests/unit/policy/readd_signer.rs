//! Removal is not permanent: a removed signer added back can settle again.

use pactlane_evaluation_policy::PolicyError;
use pactlane_interfaces::JobState;
use pactlane_tests::{Setup, BUDGET};
use soroban_sdk::{testutils::Address as _, Address};

#[test]
fn a_removed_signer_added_again_can_settle_again() {
    let s = Setup::new();
    let kernel = s.kernel();
    let policy = s.policy(&kernel.address, &Address::generate(&s.env));
    let signer = Address::generate(&s.env);
    policy.add_signer(&signer);
    let id = s.funded_job_evaluated_by(&kernel, &policy.address);
    kernel.submit(&id, &s.hash(2));

    policy.remove_signer(&signer);
    assert_eq!(
        policy.try_complete(&signer, &id, &s.hash(9)),
        Err(Ok(PolicyError::NotSigner))
    );

    policy.add_signer(&signer);
    assert!(policy.is_signer(&signer));
    policy.complete(&signer, &id, &s.hash(9));

    assert_eq!(kernel.get_job(&id).state, JobState::Completed);
    assert_eq!(s.balance(&s.provider), BUDGET);
}
