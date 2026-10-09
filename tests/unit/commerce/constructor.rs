use pactlane_commerce::CommerceKernel;
use pactlane_tests::Setup;
use soroban_sdk::{testutils::Address as _, Address};

#[test]
fn the_kernel_is_bound_to_the_token_it_was_deployed_with() {
    let s = Setup::new();
    let kernel = s.kernel();
    assert_eq!(kernel.token(), s.token.address);
}

#[test]
fn a_new_kernel_has_no_jobs() {
    let s = Setup::new();
    assert_eq!(s.kernel().job_count(), 0);
}

#[test]
// The constructor's own `decimals()` probe is what fails, not something else.
#[should_panic(expected = "\"contract call failed\", decimals")]
fn deploying_against_an_address_that_is_not_a_token_fails() {
    // T11: a kernel bound to a non-token would accept jobs it can never fund.
    let s = Setup::new();
    let not_a_token = Address::generate(&s.env);
    s.env.register(CommerceKernel, (not_a_token,));
}
