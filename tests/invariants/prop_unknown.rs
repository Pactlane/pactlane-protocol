//! No operation succeeds on a job ID that does not exist. After every step
//! of a random sequence, every job-addressed function is probed with an ID
//! that no job has (0, the next unassigned ID, one far beyond, and
//! `u64::MAX`). Each must fail with `NotFound` and change nothing.

use pactlane_commerce::CommerceKernelClient;
use pactlane_interfaces::Error;
use pactlane_tests::{Setup, BUDGET};
use proptest::prelude::*;

use crate::conservation::{all_jobs, apply, op};

fn probe(s: &Setup, kernel: &CommerceKernelClient, id: u64) -> Result<(), TestCaseError> {
    let not_found = Err(Ok(Error::NotFound));
    prop_assert_eq!(
        kernel.try_get_job(&id).map(|_| ()),
        not_found.clone(),
        "get_job({})",
        id
    );
    prop_assert_eq!(
        kernel.try_set_provider(&id, &s.provider).map(|_| ()),
        not_found.clone(),
        "set_provider({})",
        id
    );
    prop_assert_eq!(
        kernel.try_set_budget(&id, &s.client, &BUDGET).map(|_| ()),
        not_found.clone(),
        "set_budget({})",
        id
    );
    prop_assert_eq!(
        kernel.try_fund(&id, &BUDGET).map(|_| ()),
        not_found.clone(),
        "fund({})",
        id
    );
    prop_assert_eq!(
        kernel.try_submit(&id, &s.hash(2)).map(|_| ()),
        not_found.clone(),
        "submit({})",
        id
    );
    prop_assert_eq!(
        kernel.try_complete(&id, &s.hash(9)).map(|_| ()),
        not_found.clone(),
        "complete({})",
        id
    );
    prop_assert_eq!(
        kernel.try_reject(&id, &None).map(|_| ()),
        not_found.clone(),
        "reject({})",
        id
    );
    prop_assert_eq!(
        kernel.try_claim_refund(&id).map(|_| ()),
        not_found.clone(),
        "claim_refund({})",
        id
    );
    prop_assert_eq!(
        kernel.try_extend_ttl(&id).map(|_| ()),
        not_found,
        "extend_ttl({})",
        id
    );
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: std::env::var("PROPTEST_CASES").ok().and_then(|n| n.parse().ok()).unwrap_or(32),
        ..ProptestConfig::default()
    })]

    #[test]
    fn no_operation_succeeds_on_an_unknown_job_id(ops in prop::collection::vec(op(), 1..40)) {
        let s = Setup::without_snapshots();
        let kernel = s.kernel();

        for op in &ops {
            apply(&s, &kernel, op);
            let jobs = all_jobs(&kernel);
            let count = kernel.job_count();
            let escrow = s.balance(&kernel.address);

            for id in [0, count + 1, count + 1_000, u64::MAX] {
                probe(&s, &kernel, id)?;
            }

            prop_assert_eq!(kernel.job_count(), count);
            prop_assert_eq!(all_jobs(&kernel), jobs);
            prop_assert_eq!(s.balance(&kernel.address), escrow);
        }
    }
}
