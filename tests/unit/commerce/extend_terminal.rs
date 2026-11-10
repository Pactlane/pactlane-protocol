//! Terminal jobs may archive, but anyone may still renew one, for example to
//! keep its record readable. Renewal must touch nothing but the TTL.

use pactlane_commerce::DataKey;
use pactlane_interfaces::JobState;
use pactlane_tests::{Setup, JOB_DURATION};
use soroban_sdk::{testutils::storage::Persistent as _, Address};

fn job_ttl(s: &Setup, kernel: &Address, id: u64) -> (u32, u32) {
    s.env.as_contract(kernel, || {
        let storage = s.env.storage();
        (
            storage.persistent().get_ttl(&DataKey::Job(id)),
            storage.max_ttl(),
        )
    })
}

#[test]
fn extending_a_terminal_job_renews_its_ttl_and_changes_nothing_else() {
    let s = Setup::new();
    let kernel = s.kernel();
    let completed = s.submitted_job(&kernel);
    kernel.complete(&completed, &s.hash(9));
    let rejected = s.funded_job(&kernel);
    kernel.reject(&rejected, &Some(s.hash(8)));
    let expired = s.funded_job(&kernel);
    s.advance_time(JOB_DURATION);
    kernel.claim_refund(&expired);
    s.advance_time(JOB_DURATION);

    for (id, state) in [
        (completed, JobState::Completed),
        (rejected, JobState::Rejected),
        (expired, JobState::Expired),
    ] {
        let before = kernel.get_job(&id);
        assert_eq!(before.state, state);
        let balances = [
            s.balance(&s.client),
            s.balance(&s.provider),
            s.balance(&kernel.address),
        ];
        let (aged, max) = job_ttl(&s, &kernel.address, id);
        assert!(aged < max, "{state:?}: time must have aged the entry");

        kernel.extend_ttl(&id);

        assert_eq!(s.env.auths(), std::vec![], "{state:?}");
        assert!(
            s.events_of(&kernel.address).events().is_empty(),
            "{state:?}"
        );
        assert_eq!(job_ttl(&s, &kernel.address, id).0, max, "{state:?}");
        assert_eq!(kernel.get_job(&id), before, "{state:?}");
        assert_eq!(
            [
                s.balance(&s.client),
                s.balance(&s.provider),
                s.balance(&kernel.address)
            ],
            balances,
            "{state:?}"
        );
    }
    assert_eq!(kernel.job_count(), 3);
}
