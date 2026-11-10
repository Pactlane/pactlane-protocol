//! Removing one signer from a large set revokes exactly that signer.

use pactlane_evaluation_policy::PolicyError;
use pactlane_interfaces::JobState;
use pactlane_tests::Setup;
use soroban_sdk::{testutils::Address as _, Address};

#[test]
fn removing_one_of_ten_signers_leaves_the_other_nine_able_to_settle() {
    let s = Setup::without_snapshots();
    let kernel = s.kernel();
    let owner = Address::generate(&s.env);
    let policy = s.policy(&kernel.address, &owner);
    let signers: std::vec::Vec<Address> = (0..10).map(|_| Address::generate(&s.env)).collect();
    for signer in &signers {
        policy.add_signer(signer);
    }
    let removed = &signers[4];

    policy.remove_signer(removed);

    assert!(!policy.is_signer(removed));
    let refused = s.funded_job_evaluated_by(&kernel, &policy.address);
    assert_eq!(
        policy.try_reject(removed, &refused, &None),
        Err(Ok(PolicyError::NotSigner))
    );
    for (i, signer) in signers.iter().enumerate().filter(|(i, _)| *i != 4) {
        assert!(policy.is_signer(signer), "signer {i}");
        let id = s.funded_job_evaluated_by(&kernel, &policy.address);
        kernel.submit(&id, &s.hash(2));
        policy.complete(signer, &id, &s.hash(9));
        assert_eq!(kernel.get_job(&id).state, JobState::Completed, "signer {i}");
    }
    assert_eq!(kernel.get_job(&refused).state, JobState::Funded);
}
