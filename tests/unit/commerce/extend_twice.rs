//! Renewing an already-renewed entry in the same ledger is harmless: both
//! calls succeed and the TTL stays at the network maximum.

use pactlane_commerce::DataKey;
use pactlane_tests::{Setup, JOB_DURATION};
use soroban_sdk::testutils::storage::{Instance as _, Persistent as _};

#[test]
fn extending_twice_in_the_same_ledger_leaves_the_maximum_ttl() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.funded_job(&kernel);
    s.advance_time(JOB_DURATION);
    let before = kernel.get_job(&id);
    let ttls = || {
        s.env.as_contract(&kernel.address, || {
            let storage = s.env.storage();
            (
                storage.persistent().get_ttl(&DataKey::Job(id)),
                storage.instance().get_ttl(),
                storage.max_ttl(),
            )
        })
    };
    let ledger = s.env.ledger().sequence();

    kernel.extend_ttl(&id);
    let first = ttls();
    kernel.extend_ttl(&id);
    let second = ttls();

    assert_eq!(
        s.env.ledger().sequence(),
        ledger,
        "both calls in one ledger"
    );
    let (job, instance, max) = second;
    assert_eq!((job, instance), (max, max));
    assert_eq!(first, second, "the second call changed something");
    assert_eq!(kernel.get_job(&id), before);
}
