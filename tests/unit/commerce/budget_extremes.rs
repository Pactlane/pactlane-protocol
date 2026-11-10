//! The smallest and largest budgets a job can hold. The kernel never does
//! arithmetic on a budget, only moves it, so both extremes must move to the
//! base unit, with no rounding, overflow or truncation anywhere on the way.

use pactlane_commerce::CommerceKernelClient;
use pactlane_interfaces::JobState;
use pactlane_tests::{Setup, JOB_DURATION, STARTING_BALANCE};

/// Opens a job and funds it with exactly `budget`.
fn funded_with(s: &Setup, kernel: &CommerceKernelClient, budget: i128) -> u64 {
    let id = s.open_job(kernel);
    kernel.set_budget(&id, &s.client, &budget);
    kernel.fund(&id, &budget);
    assert_eq!(kernel.get_job(&id).budget, budget);
    id
}

/// Tops the client up so that it holds exactly `i128::MAX`.
fn make_client_hold_i128_max(s: &Setup) {
    s.token_admin
        .mint(&s.client, &(i128::MAX - STARTING_BALANCE));
    assert_eq!(s.balance(&s.client), i128::MAX);
}

#[test]
fn a_one_base_unit_budget_is_escrowed_and_paid_exactly() {
    let s = Setup::new();
    let kernel = s.kernel();

    let id = funded_with(&s, &kernel, 1);
    assert_eq!(s.balance(&kernel.address), 1);
    assert_eq!(s.balance(&s.client), STARTING_BALANCE - 1);

    kernel.submit(&id, &s.hash(2));
    kernel.complete(&id, &s.hash(9));

    assert_eq!(kernel.get_job(&id).state, JobState::Completed);
    assert_eq!(s.balance(&s.provider), 1);
    assert_eq!(s.balance(&s.client), STARTING_BALANCE - 1);
    assert_eq!(s.balance(&kernel.address), 0);
}

#[test]
fn a_one_base_unit_budget_is_refunded_exactly() {
    let s = Setup::new();
    let kernel = s.kernel();
    let rejected = funded_with(&s, &kernel, 1);
    let expired = funded_with(&s, &kernel, 1);

    kernel.reject(&rejected, &None);
    s.advance_time(JOB_DURATION);
    kernel.claim_refund(&expired);

    assert_eq!(s.balance(&s.client), STARTING_BALANCE);
    assert_eq!(s.balance(&s.provider), 0);
    assert_eq!(s.balance(&kernel.address), 0);
}

#[test]
fn an_i128_max_budget_is_escrowed_and_paid_in_full() {
    let s = Setup::new();
    let kernel = s.kernel();
    make_client_hold_i128_max(&s);

    let id = funded_with(&s, &kernel, i128::MAX);
    assert_eq!(s.balance(&kernel.address), i128::MAX);
    assert_eq!(s.balance(&s.client), 0);

    kernel.submit(&id, &s.hash(2));
    kernel.complete(&id, &s.hash(9));

    assert_eq!(kernel.get_job(&id).state, JobState::Completed);
    assert_eq!(s.balance(&s.provider), i128::MAX);
    assert_eq!(s.balance(&s.client), 0);
    assert_eq!(s.balance(&kernel.address), 0);
}

#[test]
fn an_i128_max_budget_is_refunded_in_full_by_rejection_and_by_expiry() {
    let s = Setup::new();
    let kernel = s.kernel();
    make_client_hold_i128_max(&s);

    // The client can afford only one such job at a time, so they run in turn.
    let rejected = funded_with(&s, &kernel, i128::MAX);
    kernel.reject(&rejected, &None);
    assert_eq!(s.balance(&s.client), i128::MAX);
    assert_eq!(s.balance(&kernel.address), 0);

    let expired = funded_with(&s, &kernel, i128::MAX);
    s.advance_time(JOB_DURATION);
    kernel.claim_refund(&expired);

    assert_eq!(kernel.get_job(&expired).state, JobState::Expired);
    assert_eq!(s.balance(&s.client), i128::MAX);
    assert_eq!(s.balance(&s.provider), 0);
    assert_eq!(s.balance(&kernel.address), 0);
}
