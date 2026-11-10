//! The policy's instance (owner, kernel, pending owner and its code) is
//! renewed to the maximum TTL by every write, so governance never lapses
//! into needing a restore.

use pactlane_evaluation_policy::EvaluationPolicyClient;
use pactlane_tests::Setup;
use soroban_sdk::{testutils::storage::Instance as _, testutils::Address as _, Address};

const WEEK: u64 = 7 * 24 * 60 * 60;

fn instance_ttl(s: &Setup, policy: &EvaluationPolicyClient) -> (u32, u32) {
    s.env.as_contract(&policy.address, || {
        (
            s.env.storage().instance().get_ttl(),
            s.env.storage().max_ttl(),
        )
    })
}

#[test]
fn every_policy_write_renews_the_instance_to_the_maximum_ttl() {
    let s = Setup::new();
    let kernel = s.kernel();
    let policy = s.policy(&kernel.address, &Address::generate(&s.env));
    let signer = Address::generate(&s.env);
    let next_owner = Address::generate(&s.env);

    let steps: [(&str, &dyn Fn()); 4] = [
        ("add_signer", &|| policy.add_signer(&signer)),
        ("remove_signer", &|| policy.remove_signer(&signer)),
        ("propose_owner", &|| policy.propose_owner(&next_owner)),
        ("accept_owner", &|| policy.accept_owner()),
    ];
    for (name, write) in steps {
        s.advance_time(WEEK);
        let (aged, max) = instance_ttl(&s, &policy);
        assert!(aged < max, "{name}: time must have aged the instance");

        write();

        assert_eq!(instance_ttl(&s, &policy).0, max, "after {name}");
    }
}
