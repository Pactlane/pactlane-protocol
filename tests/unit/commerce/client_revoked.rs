//! USDC's issuer can deauthorize a holder. A deauthorized client's funding
//! transfer is refused by the token, and the kernel must roll back the state
//! it had already written, leaving the job Open and fundable later.

use pactlane_interfaces::JobState;
use pactlane_tests::{Setup, BUDGET, STARTING_BALANCE};
use soroban_sdk::testutils::IssuerFlags;

#[test]
fn a_deauthorized_client_cannot_fund_and_the_job_stays_open() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job(&kernel);
    kernel.set_budget(&id, &s.client, &BUDGET);
    let before = kernel.get_job(&id);
    s.issuer.set_flag(IssuerFlags::RevocableFlag);
    s.token_admin.set_authorized(&s.client, &false);

    assert!(kernel.try_fund(&id, &BUDGET).is_err());

    assert_eq!(kernel.get_job(&id), before);
    assert_eq!(before.state, JobState::Open);
    assert_eq!(s.balance(&s.client), STARTING_BALANCE);
    assert_eq!(s.balance(&kernel.address), 0);

    // Once re-authorized, the same agreement funds normally.
    s.token_admin.set_authorized(&s.client, &true);
    kernel.fund(&id, &BUDGET);
    assert_eq!(kernel.get_job(&id).state, JobState::Funded);
    assert_eq!(s.balance(&kernel.address), BUDGET);
}
