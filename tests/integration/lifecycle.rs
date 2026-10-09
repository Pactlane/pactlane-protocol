//! Whole job lifecycles with both contracts running as compiled Wasm, the
//! policy calling the kernel inside the VM exactly as it would on chain.
//! Every operation must also fit well within testnet's per-transaction limits.

use pactlane_commerce::CommerceKernelClient;
use pactlane_evaluation_policy::EvaluationPolicyClient;
use pactlane_interfaces::JobState;
use pactlane_tests::{Setup, BUDGET, JOB_DURATION, STARTING_BALANCE};
use soroban_sdk::{testutils::Address as _, Address};

use crate::release_wasm;

struct Deployment {
    s: Setup,
    kernel: CommerceKernelClient<'static>,
    policy: EvaluationPolicyClient<'static>,
    signer: Address,
}

fn deploy() -> Deployment {
    let s = Setup::new();
    let kernel = s.kernel_from_wasm(&release_wasm("pactlane-commerce"));
    let policy = s.policy_from_wasm(
        &release_wasm("pactlane-evaluation-policy"),
        &kernel.address,
        &Address::generate(&s.env),
    );
    let signer = Address::generate(&s.env);
    policy.add_signer(&signer);
    Deployment {
        s,
        kernel,
        policy,
        signer,
    }
}

/// Opens and funds a job evaluated by the policy, checking each step's
/// resource use.
fn funded_job(d: &Deployment) -> u64 {
    let s = &d.s;
    let id = d.kernel.create_job(
        &s.client,
        &Some(s.provider.clone()),
        &d.policy.address,
        &(s.now() + JOB_DURATION),
        &s.hash(1),
    );
    s.assert_well_within_tx_limits("create_job");
    d.kernel.set_budget(&id, &s.provider, &BUDGET);
    s.assert_well_within_tx_limits("set_budget");
    d.kernel.fund(&id, &BUDGET);
    s.assert_well_within_tx_limits("fund");
    id
}

#[test]
fn a_job_completed_through_the_policy_pays_the_provider() {
    let d = deploy();
    let id = funded_job(&d);
    d.kernel.submit(&id, &d.s.hash(2));
    d.s.assert_well_within_tx_limits("submit");

    d.policy.complete(&d.signer, &id, &d.s.hash(9));
    d.s.assert_well_within_tx_limits("policy complete");

    assert_eq!(d.kernel.get_job(&id).state, JobState::Completed);
    assert_eq!(d.s.balance(&d.s.provider), BUDGET);
    assert_eq!(d.s.balance(&d.kernel.address), 0);
}

#[test]
fn a_job_rejected_through_the_policy_refunds_the_client() {
    let d = deploy();
    let id = funded_job(&d);
    d.kernel.submit(&id, &d.s.hash(2));

    d.policy.reject(&d.signer, &id, &Some(d.s.hash(8)));
    d.s.assert_well_within_tx_limits("policy reject");

    assert_eq!(d.kernel.get_job(&id).state, JobState::Rejected);
    assert_eq!(d.s.balance(&d.s.client), STARTING_BALANCE);
    assert_eq!(d.s.balance(&d.kernel.address), 0);
}

#[test]
fn an_unreviewed_job_is_refunded_after_expiry() {
    let d = deploy();
    let id = funded_job(&d);
    d.kernel.submit(&id, &d.s.hash(2));
    d.s.advance_time(JOB_DURATION);

    d.kernel.claim_refund(&id);
    d.s.assert_well_within_tx_limits("claim_refund");

    assert_eq!(d.kernel.get_job(&id).state, JobState::Expired);
    assert_eq!(d.s.balance(&d.s.client), STARTING_BALANCE);
}

#[test]
fn an_open_job_cancelled_by_its_client_moves_nothing() {
    let d = deploy();
    let id = d.kernel.create_job(
        &d.s.client,
        &None,
        &d.policy.address,
        &(d.s.now() + JOB_DURATION),
        &d.s.hash(1),
    );
    d.kernel.set_provider(&id, &d.s.provider);
    d.s.assert_well_within_tx_limits("set_provider");

    d.kernel.reject(&id, &None);
    d.s.assert_well_within_tx_limits("client reject");

    assert_eq!(d.kernel.get_job(&id).state, JobState::Rejected);
    assert_eq!(d.s.balance(&d.s.client), STARTING_BALANCE);
}
