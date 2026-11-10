//! `job_count` only grows: each step leaves it unchanged or adds exactly one,
//! only a `create_job` can add, and every ID up to it names a job.

use pactlane_tests::Setup;
use proptest::prelude::*;

use crate::conservation::{all_jobs, apply, op, Op};

proptest! {
    #![proptest_config(ProptestConfig {
        cases: std::env::var("PROPTEST_CASES").ok().and_then(|n| n.parse().ok()).unwrap_or(64),
        ..ProptestConfig::default()
    })]

    #[test]
    fn job_count_is_monotonic(ops in prop::collection::vec(op(), 1..60)) {
        let s = Setup::without_snapshots();
        let kernel = s.kernel();
        let mut count = kernel.job_count();

        for op in &ops {
            apply(&s, &kernel, op);
            let next = kernel.job_count();
            prop_assert!(next == count || next == count + 1, "{} -> {} after {:?}", count, next, op);
            if next > count {
                prop_assert!(matches!(op, Op::Create { .. }), "{:?} created a job", op);
            }
            prop_assert_eq!(all_jobs(&kernel).len() as u64, next);
            count = next;
        }
    }
}
