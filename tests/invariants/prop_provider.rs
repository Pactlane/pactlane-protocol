//! A job's provider is set at most once: once it is `Some`, no operation in
//! any random sequence replaces or clears it.

use std::collections::BTreeMap;

use pactlane_interfaces::{Job, JobState};
use pactlane_tests::Setup;
use proptest::prelude::*;

use crate::conservation::{all_jobs, apply, op};

fn check(before: &BTreeMap<u64, Job>, after: &BTreeMap<u64, Job>) -> Result<(), TestCaseError> {
    for (id, old) in before {
        if old.provider.is_some() {
            prop_assert_eq!(
                &after[id].provider,
                &old.provider,
                "provider of job {} changed",
                id
            );
        }
    }
    // A provider can only be assigned while the job is still Open.
    for (id, job) in after {
        if job.provider.is_none() {
            prop_assert!(
                matches!(job.state, JobState::Open | JobState::Rejected),
                "job {} reached {:?} without a provider",
                id,
                job.state
            );
        }
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: std::env::var("PROPTEST_CASES").ok().and_then(|n| n.parse().ok()).unwrap_or(64),
        ..ProptestConfig::default()
    })]

    #[test]
    fn a_provider_never_changes_once_set(ops in prop::collection::vec(op(), 1..60)) {
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
