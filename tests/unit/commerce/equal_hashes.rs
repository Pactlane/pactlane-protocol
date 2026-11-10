//! The kernel never compares hashes with each other. An evaluator may cite
//! the work hash itself as its evidence, and completion must still succeed.

use pactlane_interfaces::JobState;
use pactlane_tests::{Setup, BUDGET};

#[test]
fn completion_succeeds_when_the_reason_hash_equals_the_work_hash() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.submitted_job(&kernel);
    let work = kernel.get_job(&id).work_hash.unwrap();

    kernel.complete(&id, &work);

    let job = kernel.get_job(&id);
    assert_eq!(job.state, JobState::Completed);
    assert_eq!(job.reason, Some(work.clone()));
    assert_eq!(job.work_hash, Some(work));
    assert_eq!(s.balance(&s.provider), BUDGET);
}

#[test]
fn spec_work_and_reason_may_all_be_the_same_hash() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.funded_job(&kernel);
    let spec = kernel.get_job(&id).spec_hash;

    kernel.submit(&id, &spec);
    kernel.complete(&id, &spec);

    assert_eq!(kernel.get_job(&id).state, JobState::Completed);
    assert_eq!(s.balance(&s.provider), BUDGET);
}
