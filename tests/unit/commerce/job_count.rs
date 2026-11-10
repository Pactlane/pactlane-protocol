//! `job_count` is the number of jobs ever created, so only `create_job` may
//! move it. Every other function, succeeding or refusing, leaves it alone.

use pactlane_interfaces::Error;
use pactlane_tests::{Setup, BUDGET, JOB_DURATION};

#[test]
fn no_function_but_create_job_moves_job_count() {
    let s = Setup::new();
    let kernel = s.kernel();
    let bare = s.open_job_without_provider(&kernel);
    let paid = s.open_job(&kernel);
    let rejected = s.funded_job(&kernel);
    let expired = s.funded_job(&kernel);
    let cancelled = s.open_job(&kernel);
    let count = kernel.job_count();
    assert_eq!(count, 5);

    let unchanged = |call: &str| assert_eq!(kernel.job_count(), count, "after {call}");

    kernel.set_provider(&bare, &s.provider);
    unchanged("set_provider");
    kernel.set_budget(&paid, &s.provider, &BUDGET);
    unchanged("set_budget");
    kernel.fund(&paid, &BUDGET);
    unchanged("fund");
    kernel.submit(&paid, &s.hash(2));
    unchanged("submit");
    kernel.complete(&paid, &s.hash(9));
    unchanged("complete");
    kernel.reject(&rejected, &None);
    unchanged("evaluator reject");
    kernel.reject(&cancelled, &None);
    unchanged("client cancel");
    kernel.extend_ttl(&paid);
    unchanged("extend_ttl");
    kernel.get_job(&paid);
    unchanged("get_job");
    kernel.token();
    unchanged("token");
    s.advance_time(JOB_DURATION);
    kernel.claim_refund(&expired);
    unchanged("claim_refund");

    // Refusals leave it alone too.
    assert_eq!(kernel.try_fund(&paid, &BUDGET), Err(Ok(Error::BadState)));
    unchanged("refused fund");
    assert_eq!(kernel.try_get_job(&99), Err(Ok(Error::NotFound)));
    unchanged("refused get_job");
}
