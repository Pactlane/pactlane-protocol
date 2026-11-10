//! A job's `expires_at` is fixed by `create_job`: no later operation in any
//! random sequence moves it, so the expiry both parties agreed is the one
//! that applies.

use std::collections::BTreeMap;

use pactlane_interfaces::Job;
use pactlane_tests::Setup;
use proptest::prelude::*;

use crate::conservation::{all_jobs, apply, op};

fn check(before: &BTreeMap<u64, Job>, after: &BTreeMap<u64, Job>) -> Result<(), TestCaseError> {
    for (id, old) in before {
        prop_assert_eq!(
            &after[id].expires_at,
            &old.expires_at,
            "expires_at of job {} changed",
            id
        );
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: std::env::var("PROPTEST_CASES").ok().and_then(|n| n.parse().ok()).unwrap_or(64),
        ..ProptestConfig::default()
    })]

    #[test]
    fn an_expiry_never_changes_after_creation(ops in prop::collection::vec(op(), 1..60)) {
        let s = Setup::without_snapshots();
        let kernel = s.kernel();
        let mut jobs = all_jobs(&kernel);

        for op in &ops {
            apply(&s, &kernel, op);
            let next = all_jobs(&kernel);
            check(&jobs, &next)?;
            jobs = next;
        }
    }
}
