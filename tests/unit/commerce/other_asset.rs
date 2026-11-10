//! The kernel is not tied to the fixture token: bound to any other asset
//! contract, it escrows and pays in that asset alone.

use pactlane_commerce::{CommerceKernel, CommerceKernelClient};
use pactlane_interfaces::JobState;
use pactlane_tests::{Setup, BUDGET, STARTING_BALANCE};
use soroban_sdk::{
    testutils::Address as _,
    token::{StellarAssetClient, TokenClient},
    Address,
};

#[test]
fn a_kernel_bound_to_a_second_asset_runs_the_happy_path_in_that_asset() {
    let s = Setup::new();
    let asset = s
        .env
        .register_stellar_asset_contract_v2(Address::generate(&s.env));
    let other = TokenClient::new(&s.env, &asset.address());
    StellarAssetClient::new(&s.env, &asset.address()).mint(&s.client, &STARTING_BALANCE);
    let id = s
        .env
        .register(CommerceKernel, (asset.address(), s.admin.clone()));
    let kernel = CommerceKernelClient::new(&s.env, &id);
    assert_eq!(kernel.token(), asset.address());

    let job = s.submitted_job(&kernel);
    kernel.complete(&job, &s.hash(9));

    assert_eq!(kernel.get_job(&job).state, JobState::Completed);
    assert_eq!(other.balance(&s.provider), BUDGET);
    assert_eq!(other.balance(&s.client), STARTING_BALANCE - BUDGET);
    assert_eq!(other.balance(&kernel.address), 0);
    // The fixture token never moved.
    assert_eq!(s.balance(&s.client), STARTING_BALANCE);
    assert_eq!(s.balance(&s.provider), 0);
}
