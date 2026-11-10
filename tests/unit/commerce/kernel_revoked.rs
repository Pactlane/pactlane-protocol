//! If the issuer deauthorizes the kernel's own balance, no settlement can
//! move escrow. Each must fail as a whole, leaving the job as it was, and
//! succeed once the kernel is re-authorized.

use pactlane_interfaces::JobState;
use pactlane_tests::{Setup, BUDGET, JOB_DURATION, STARTING_BALANCE};
use soroban_sdk::testutils::IssuerFlags;

#[test]
fn a_deauthorized_kernel_blocks_completion_atomically_until_reauthorized() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.submitted_job(&kernel);
    let before = kernel.get_job(&id);
    s.issuer.set_flag(IssuerFlags::RevocableFlag);
    s.token_admin.set_authorized(&kernel.address, &false);

    assert!(kernel.try_complete(&id, &s.hash(9)).is_err());

    assert_eq!(kernel.get_job(&id), before);
    assert_eq!(s.balance(&kernel.address), BUDGET);
    assert_eq!(s.balance(&s.provider), 0);

    s.token_admin.set_authorized(&kernel.address, &true);
    kernel.complete(&id, &s.hash(9));
    assert_eq!(kernel.get_job(&id).state, JobState::Completed);
    assert_eq!(s.balance(&s.provider), BUDGET);
}

#[test]
fn a_deauthorized_kernel_blocks_refunds_atomically_until_reauthorized() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.funded_job(&kernel);
    s.advance_time(JOB_DURATION);
    let before = kernel.get_job(&id);
    s.issuer.set_flag(IssuerFlags::RevocableFlag);
    s.token_admin.set_authorized(&kernel.address, &false);

    assert!(kernel.try_claim_refund(&id).is_err());

    assert_eq!(kernel.get_job(&id), before);
    assert_eq!(before.state, JobState::Funded);
    assert_eq!(s.balance(&kernel.address), BUDGET);

    s.token_admin.set_authorized(&kernel.address, &true);
    kernel.claim_refund(&id);
    assert_eq!(kernel.get_job(&id).state, JobState::Expired);
    assert_eq!(s.balance(&s.client), STARTING_BALANCE);
}
