//! `work_hash` is set exactly when the provider has submitted: never on an
//! Open or Funded job, always on a Submitted or Completed one, kept by a job
//! rejected or expired after submission, and never changed once set.

use std::collections::BTreeMap;

use pactlane_interfaces::{Job, JobState};
use pactlane_tests::Setup;
use proptest::prelude::*;

use crate::conservation::{all_jobs, apply, op};

fn check(before: &BTreeMap<u64, Job>, after: &BTreeMap<u64, Job>) -> Result<(), TestCaseError> {
    for (id, job) in after {
        let submitted_earlier = before.get(id).is_some_and(|old| old.work_hash.is_some());
        let expected = match job.state {
            JobState::Open | JobState::Funded => false,
            JobState::Submitted | JobState::Completed => true,
            JobState::Rejected | JobState::Expired => submitted_earlier,
        };
        prop_assert_eq!(
            job.work_hash.is_some(),
            expected,
            "job {} in {:?} has work_hash {:?}",
            id,
            job.state,
            job.work_hash
        );
        if let Some(old) = before.get(id).and_then(|old| old.work_hash.as_ref()) {
            prop_assert_eq!(
                job.work_hash.as_ref(),
                Some(old),
                "work_hash of job {} changed",
                id
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
    fn work_hash_is_set_if_and_only_if_work_was_submitted(ops in prop::collection::vec(op(), 1..60)) {
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
