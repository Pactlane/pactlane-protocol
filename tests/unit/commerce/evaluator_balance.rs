//! The evaluator decides where escrow goes but never receives or pays any
//! of it, whatever the outcome and however much it holds itself.

use pactlane_tests::{Setup, JOB_DURATION, USDC};

#[test]
fn an_evaluator_holding_tokens_is_never_paid_or_charged() {
    let s = Setup::new();
    let held = 50 * USDC;
    s.token_admin.mint(&s.evaluator, &held);
    let kernel = s.kernel();
    let unchanged = |step: &str| assert_eq!(s.balance(&s.evaluator), held, "after {step}");

    let paid = s.submitted_job(&kernel);
    let rejected = s.submitted_job(&kernel);
    let expired = s.funded_job(&kernel);
    let cancelled = s.open_job(&kernel);
    unchanged("funding");

    kernel.complete(&paid, &s.hash(9));
    unchanged("complete");
    kernel.reject(&rejected, &None);
    unchanged("evaluator reject");
    kernel.reject(&cancelled, &None);
    unchanged("client cancel");
    s.advance_time(JOB_DURATION);
    kernel.claim_refund(&expired);
    unchanged("claim_refund");
    assert_eq!(s.balance(&kernel.address), 0);
}
