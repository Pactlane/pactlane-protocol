//! Funding and settlement attacks not already covered next to the function
//! they target (budget race, double funding and double settlement live in
//! `fund.rs`, `complete.rs`, `reject.rs` and `claim_refund.rs`).

use pactlane_interfaces::{Error, JobState};
use pactlane_tests::{Setup, BUDGET, JOB_DURATION, STARTING_BALANCE, USDC};
use soroban_sdk::{
    testutils::{Address as _, IssuerFlags},
    token::{StellarAssetClient, TokenClient},
    Address,
};

#[test]
fn funding_leaves_no_standing_allowance() {
    // The kernel pulls funds with a transfer the client signs once, not with
    // an approval, so there is nothing left over for anyone to drain later.
    let s = Setup::new();
    let kernel = s.kernel();

    s.funded_job(&kernel);

    assert_eq!(s.token.allowance(&s.client, &kernel.address), 0);
}

#[test]
fn a_kernel_only_ever_moves_its_own_token() {
    // T10: a client holding several assets cannot be charged in the wrong one.
    let s = Setup::new();
    let other_asset = s
        .env
        .register_stellar_asset_contract_v2(Address::generate(&s.env));
    let other = TokenClient::new(&s.env, &other_asset.address());
    StellarAssetClient::new(&s.env, &other_asset.address()).mint(&s.client, &STARTING_BALANCE);
    let kernel = s.kernel();

    let id = s.submitted_job(&kernel);
    kernel.complete(&id, &s.hash(9));

    assert_eq!(kernel.token(), s.token.address);
    assert_eq!(s.balance(&s.provider), BUDGET);
    assert_eq!(other.balance(&s.client), STARTING_BALANCE);
    assert_eq!(other.balance(&s.provider), 0);
    assert_eq!(other.balance(&kernel.address), 0);
}

#[test]
fn tokens_sent_straight_to_the_kernel_change_no_payout() {
    // Payouts come from each job's recorded budget, never from the kernel's
    // balance, so a donation cannot inflate what anyone receives.
    let s = Setup::new();
    let kernel = s.kernel();
    let paid = s.submitted_job(&kernel);
    let refunded = s.funded_job(&kernel);
    s.token_admin.mint(&kernel.address, &(100 * USDC));

    kernel.complete(&paid, &s.hash(9));
    kernel.reject(&refunded, &None);

    assert_eq!(s.balance(&s.provider), BUDGET);
    assert_eq!(s.balance(&s.client), STARTING_BALANCE - BUDGET);
    // With no admin, the donation stays in the kernel for good.
    assert_eq!(s.balance(&kernel.address), 100 * USDC);
}

#[test]
fn a_budget_lowered_after_the_client_agreed_is_refused_too() {
    // The budget race protects both directions: the client funds exactly
    // what it agreed, and the provider cannot be underpaid by a late change.
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job(&kernel);
    kernel.set_budget(&id, &s.provider, &BUDGET);
    kernel.set_budget(&id, &s.client, &(BUDGET - 1));

    assert_eq!(
        kernel.try_fund(&id, &BUDGET),
        Err(Ok(Error::BudgetMismatch))
    );
}

#[test]
fn a_refused_payout_changes_nothing_and_the_client_can_still_recover() {
    // T18: if the token will not deliver to the provider (here the issuer
    // has deauthorized its balance), completion must fail as a whole rather
    // than mark the job paid. After expiry the client recovers the escrow.
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.submitted_job(&kernel);
    // Circle's USDC issuer can revoke authorization; the fixture asset gets
    // the same power so the scenario is realistic.
    s.issuer.set_flag(IssuerFlags::RevocableFlag);
    s.token_admin.set_authorized(&s.provider, &false);

    assert!(kernel.try_complete(&id, &s.hash(9)).is_err());
    assert_eq!(kernel.get_job(&id).state, JobState::Submitted);
    assert_eq!(s.balance(&kernel.address), BUDGET);

    s.advance_time(JOB_DURATION);
    kernel.claim_refund(&id);
    assert_eq!(s.balance(&s.client), STARTING_BALANCE);
}

#[test]
fn many_jobs_settle_independently() {
    // Each outcome moves exactly its own job's budget to exactly its own
    // party; the escrow ends empty.
    let s = Setup::new();
    let kernel = s.kernel();
    let paid = s.submitted_job(&kernel);
    let rejected = s.submitted_job(&kernel);
    let expired = s.funded_job(&kernel);
    let cancelled = s.open_job(&kernel);
    assert_eq!(s.balance(&kernel.address), 3 * BUDGET);

    kernel.complete(&paid, &s.hash(9));
    kernel.reject(&rejected, &None);
    kernel.reject(&cancelled, &None);
    s.advance_time(JOB_DURATION);
    kernel.claim_refund(&expired);

    assert_eq!(s.balance(&s.provider), BUDGET);
    assert_eq!(s.balance(&s.client), STARTING_BALANCE - BUDGET);
    assert_eq!(s.balance(&kernel.address), 0);
}
