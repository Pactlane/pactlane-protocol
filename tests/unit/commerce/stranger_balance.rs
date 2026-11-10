//! Anyone may trigger a refund, but the money only ever goes to the job's
//! client. An outsider holding tokens who claims refunds gains and loses
//! nothing.

use pactlane_tests::{Setup, BUDGET, JOB_DURATION, STARTING_BALANCE, USDC};

#[test]
fn a_stranger_holding_tokens_who_claims_refunds_is_never_paid_or_charged() {
    let s = Setup::new();
    let held = 50 * USDC;
    s.token_admin.mint(&s.stranger, &held);
    let kernel = s.kernel();
    let funded = s.funded_job(&kernel);
    let submitted = s.submitted_job(&kernel);
    let paid = s.submitted_job(&kernel);
    kernel.complete(&paid, &s.hash(9));
    assert_eq!(s.balance(&s.stranger), held);

    s.advance_time(JOB_DURATION);
    for id in [funded, submitted] {
        // Submitted by the stranger: claim_refund needs no one's signature.
        kernel.claim_refund(&id);
        assert_eq!(s.env.auths(), std::vec![]);
        assert_eq!(s.balance(&s.stranger), held);
    }

    assert_eq!(s.balance(&s.client), STARTING_BALANCE - BUDGET);
    assert_eq!(s.balance(&s.provider), BUDGET);
    assert_eq!(s.balance(&kernel.address), 0);
}
