//! In-place upgrades (threat T17): one key per contract may replace its
//! code, keeping the address integrators pin, every job and the escrow.
//! Run against the release Wasm, since only uploaded code can be swapped in.

use pactlane_interfaces::{JobState, Upgraded};
use pactlane_tests::{Setup, BUDGET, STARTING_BALANCE};
use soroban_sdk::{Event, IntoVal};

use crate::release_wasm;

#[test]
fn the_admin_upgrades_the_kernel_in_place_keeping_jobs_and_escrow() {
    let s = Setup::new();
    let wasm = release_wasm("pactlane-commerce");
    let kernel = s.kernel_from_wasm(&wasm);
    let open = s.open_job(&kernel);
    let submitted = s.submitted_job(&kernel);
    let before = (kernel.get_job(&open), kernel.get_job(&submitted));
    let hash = s.env.deployer().upload_contract_wasm(wasm.as_slice());

    kernel.upgrade(&hash);

    s.assert_authorized_by(&s.admin, &kernel.address, "upgrade");
    assert_eq!(
        s.events_of(&kernel.address),
        [Upgraded {
            admin: s.admin.clone(),
            wasm_hash: hash
        }
        .to_xdr(&s.env, &kernel.address)]
    );
    assert_eq!((kernel.get_job(&open), kernel.get_job(&submitted)), before);
    assert_eq!(kernel.job_count(), 2);
    assert_eq!(kernel.token(), s.token.address);
    assert_eq!(kernel.admin(), s.admin);
    assert_eq!(s.balance(&kernel.address), BUDGET);

    // Jobs funded before the upgrade settle under the new code.
    kernel.complete(&submitted, &s.hash(9));
    assert_eq!(kernel.get_job(&submitted).state, JobState::Completed);
    assert_eq!(s.balance(&s.provider), BUDGET);
    assert_eq!(s.balance(&s.client), STARTING_BALANCE - BUDGET);
}

#[test]
fn an_upgrade_really_replaces_the_code() {
    // Swapping in unrelated Wasm (the policy's) removes the kernel's own
    // functions, proving the call changes what runs at the address.
    let s = Setup::new();
    let kernel = s.kernel_from_wasm(&release_wasm("pactlane-commerce"));
    let other = s
        .env
        .deployer()
        .upload_contract_wasm(release_wasm("pactlane-evaluation-policy").as_slice());

    kernel.upgrade(&other);

    assert!(kernel.try_job_count().is_err());
}

#[test]
fn nobody_but_the_admin_can_upgrade_the_kernel() {
    let s = Setup::new();
    let wasm = release_wasm("pactlane-commerce");
    let kernel = s.kernel_from_wasm(&wasm);
    s.funded_job(&kernel);
    let hash = s.env.deployer().upload_contract_wasm(wasm.as_slice());

    for who in [&s.client, &s.provider, &s.evaluator, &s.stranger] {
        s.sign_only_as(
            who,
            &kernel.address,
            "upgrade",
            (hash.clone(),).into_val(&s.env),
            &[],
        );
        assert!(kernel.try_upgrade(&hash).is_err(), "{who:?} upgraded");
    }
    s.env.mock_all_auths();
    assert_eq!(s.balance(&kernel.address), BUDGET);
}

#[test]
fn the_policy_owner_upgrades_the_policy_in_place_keeping_its_signers() {
    let s = Setup::new();
    let kernel = s.kernel_from_wasm(&release_wasm("pactlane-commerce"));
    let wasm = release_wasm("pactlane-evaluation-policy");
    let policy = s.policy_from_wasm(&wasm, &kernel.address, &s.admin);
    policy.add_signer(&s.stranger);
    let hash = s.env.deployer().upload_contract_wasm(wasm.as_slice());

    policy.upgrade(&hash);

    s.assert_authorized_by(&s.admin, &policy.address, "upgrade");
    assert!(policy.is_signer(&s.stranger));
    assert_eq!(policy.owner(), s.admin);
    assert_eq!(policy.kernel(), kernel.address);
}

#[test]
fn nobody_but_the_owner_can_upgrade_the_policy() {
    let s = Setup::new();
    let kernel = s.kernel_from_wasm(&release_wasm("pactlane-commerce"));
    let wasm = release_wasm("pactlane-evaluation-policy");
    let policy = s.policy_from_wasm(&wasm, &kernel.address, &s.admin);
    policy.add_signer(&s.stranger);
    let hash = s.env.deployer().upload_contract_wasm(wasm.as_slice());

    // A signer can settle jobs, but must not be able to replace the code.
    s.sign_only_as(
        &s.stranger,
        &policy.address,
        "upgrade",
        (hash.clone(),).into_val(&s.env),
        &[],
    );
    assert!(policy.try_upgrade(&hash).is_err());
}
