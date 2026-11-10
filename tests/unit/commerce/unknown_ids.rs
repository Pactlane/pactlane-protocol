//! IDs start at 1, so 0 is never a job, and `u64::MAX` would take 2^64 - 1
//! creations to reach. Both must read as `NotFound`, even while real jobs
//! exist, rather than wrapping or aliasing onto one of them.

use pactlane_interfaces::Error;
use pactlane_tests::{Setup, BUDGET};

#[test]
fn get_job_of_zero_or_u64_max_is_not_found() {
    let s = Setup::new();
    let kernel = s.kernel();

    for id in [0, u64::MAX] {
        assert_eq!(
            kernel.try_get_job(&id),
            Err(Ok(Error::NotFound)),
            "empty kernel, id {id}"
        );
    }

    s.open_job(&kernel);
    s.funded_job(&kernel);

    for id in [0, u64::MAX] {
        assert_eq!(
            kernel.try_get_job(&id),
            Err(Ok(Error::NotFound)),
            "two jobs, id {id}"
        );
    }
    // The next ID is not found until it is created either.
    assert_eq!(kernel.try_get_job(&3), Err(Ok(Error::NotFound)));
}

#[test]
fn every_job_function_refuses_the_boundary_ids_as_not_found() {
    let s = Setup::new();
    let kernel = s.kernel();
    s.funded_job(&kernel);

    for id in [0, u64::MAX] {
        let refusals = [
            ("set_provider", kernel.try_set_provider(&id, &s.provider)),
            ("set_budget", kernel.try_set_budget(&id, &s.client, &BUDGET)),
            ("fund", kernel.try_fund(&id, &BUDGET)),
            ("submit", kernel.try_submit(&id, &s.hash(2))),
            ("complete", kernel.try_complete(&id, &s.hash(9))),
            ("reject", kernel.try_reject(&id, &None)),
            ("claim_refund", kernel.try_claim_refund(&id)),
            ("extend_ttl", kernel.try_extend_ttl(&id)),
        ];
        for (name, outcome) in refusals {
            assert_eq!(outcome, Err(Ok(Error::NotFound)), "{name}({id})");
        }
    }
    assert_eq!(s.balance(&kernel.address), BUDGET);
    assert_eq!(kernel.job_count(), 1);
}
