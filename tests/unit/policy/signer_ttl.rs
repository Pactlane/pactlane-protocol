//! An active signer's entry is renewed every time it settles, so a signer
//! in regular use never has to be restored before it can act.

use pactlane_evaluation_policy::DataKey;
use pactlane_tests::{Setup, JOB_DURATION};
use soroban_sdk::{testutils::storage::Persistent as _, testutils::Address as _, Address};

fn signer_ttl(s: &Setup, policy: &Address, signer: &Address) -> (u32, u32) {
    s.env.as_contract(policy, || {
        let storage = s.env.storage();
        (
            storage
                .persistent()
                .get_ttl(&DataKey::Signer(signer.clone())),
            storage.max_ttl(),
        )
    })
}

#[test]
fn settling_renews_the_signers_entry_to_the_maximum_ttl() {
    let s = Setup::new();
    let kernel = s.kernel();
    let policy = s.policy(&kernel.address, &Address::generate(&s.env));
    let signer = Address::generate(&s.env);
    policy.add_signer(&signer);
    let id = s.funded_job_evaluated_by(&kernel, &policy.address);
    kernel.submit(&id, &s.hash(2));
    s.advance_time(JOB_DURATION - 1);
    let (aged, max) = signer_ttl(&s, &policy.address, &signer);
    assert!(aged < max, "time must have aged the signer entry");

    policy.complete(&signer, &id, &s.hash(9));

    assert_eq!(signer_ttl(&s, &policy.address, &signer).0, max);
}
