//! Terminal states are absorbing (specs/INVARIANTS.md, invariant 3): once a
//! job is Completed, Rejected or Expired, no operation in any random
//! sequence changes a single field of it.

use std::collections::BTreeMap;

use pactlane_interfaces::Job;
use pactlane_tests::Setup;
use proptest::prelude::*;

use crate::conservation::{all_jobs, apply, op};

fn check(before: &BTreeMap<u64, Job>, after: &BTreeMap<u64, Job>) -> Result<(), TestCaseError> {
    for (id, old) in before {
        if old.state.is_terminal() {
            prop_assert_eq!(&after[id], old, "terminal job {} changed", id);
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
    fn terminal_jobs_are_never_modified(ops in prop::collection::vec(op(), 1..60)) {
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
