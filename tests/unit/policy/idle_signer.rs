//! Signer membership has nothing to do with jobs: an address can be added
//! before any job names the policy, and is ready to settle once one does.

use pactlane_interfaces::JobState;
use pactlane_tests::Setup;
use soroban_sdk::{testutils::Address as _, Address};

#[test]
fn a_signer_can_be_added_before_the_policy_evaluates_any_job() {
    let s = Setup::new();
    let kernel = s.kernel();
    let policy = s.policy(&kernel.address, &Address::generate(&s.env));
    let signer = Address::generate(&s.env);
    assert_eq!(kernel.job_count(), 0);

    policy.add_signer(&signer);

    assert!(policy.is_signer(&signer));
    let id = s.funded_job_evaluated_by(&kernel, &policy.address);
    policy.reject(&signer, &id, &None);
    assert_eq!(kernel.get_job(&id).state, JobState::Rejected);
}
