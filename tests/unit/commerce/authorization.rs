//! The authorization matrix (threat T9): every signing function is called
//! once per possible signer, with real authorization instead of mocked, and
//! only the role the spec names may succeed.
//!
//! The rest of the suite mocks all authorizations and checks who the kernel
//! *asked* for. This file checks that nobody else's signature is accepted.

use pactlane_interfaces::{JobState, JobState::*};
use pactlane_tests::{Setup, BUDGET, JOB_DURATION};
use soroban_sdk::{
    testutils::{MockAuth, MockAuthInvoke},
    Address, IntoVal, Val, Vec,
};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Role {
    Client,
    Provider,
    Evaluator,
    Stranger,
}

const ROLES: [Role; 4] = [
    Role::Client,
    Role::Provider,
    Role::Evaluator,
    Role::Stranger,
];

fn address(s: &Setup, role: Role) -> Address {
    match role {
        Role::Client => s.client.clone(),
        Role::Provider => s.provider.clone(),
        Role::Evaluator => s.evaluator.clone(),
        Role::Stranger => s.stranger.clone(),
    }
}

/// Replaces mocked authorization with exactly one signature: `role`'s, for
/// `function(args)` on `contract`, with the given sub-invocations.
fn sign_only_as(
    s: &Setup,
    role: Role,
    contract: &Address,
    function: &str,
    args: Vec<Val>,
    sub_invokes: &[MockAuthInvoke],
) {
    s.env.mock_auths(&[MockAuth {
        address: &address(s, role),
        invoke: &MockAuthInvoke {
            contract,
            fn_name: function,
            args,
            sub_invokes,
        },
    }]);
}

/// Asserts the outcome of one matrix cell. A refused signer must fail on
/// authorization (a host error, `Err(Err(_))`), not on a contract check,
/// and must change nothing.
fn assert_cell<T: core::fmt::Debug, E: core::fmt::Debug, H: core::fmt::Debug>(
    result: Result<T, Result<E, H>>,
    allowed: bool,
    cell: &str,
) {
    if allowed {
        assert!(result.is_ok(), "{cell}: should succeed, got {result:?}");
    } else {
        assert!(
            matches!(result, Err(Err(_))),
            "{cell}: should be refused for lack of authorization, got {result:?}"
        );
    }
}

#[test]
fn only_the_client_can_create_a_job() {
    for role in ROLES {
        let s = Setup::new();
        let kernel = s.kernel();
        let expires_at = s.now() + JOB_DURATION;
        let args = (
            s.client.clone(),
            Some(s.provider.clone()),
            s.evaluator.clone(),
            expires_at,
            s.hash(1),
        );
        sign_only_as(
            &s,
            role,
            &kernel.address,
            "create_job",
            args.clone().into_val(&s.env),
            &[],
        );

        let result = kernel.try_create_job(&args.0, &args.1, &args.2, &args.3, &args.4);

        assert_cell(
            result,
            role == Role::Client,
            &format!("create_job by {role:?}"),
        );
        if role != Role::Client {
            assert_eq!(kernel.job_count(), 0);
        }
    }
}

#[test]
fn only_the_client_can_assign_the_provider() {
    for role in ROLES {
        let s = Setup::new();
        let kernel = s.kernel();
        let id = s.open_job_without_provider(&kernel);
        sign_only_as(
            &s,
            role,
            &kernel.address,
            "set_provider",
            (id, s.provider.clone()).into_val(&s.env),
            &[],
        );

        let result = kernel.try_set_provider(&id, &s.provider);

        assert_cell(
            result,
            role == Role::Client,
            &format!("set_provider by {role:?}"),
        );
        if role != Role::Client {
            assert_eq!(kernel.get_job(&id).provider, None);
        }
    }
}

#[test]
fn a_budget_is_proposed_only_with_the_named_actors_signature() {
    // The actor is an argument, so the signature must match it: signing as
    // the provider must not let anyone propose a budget "as the client".
    for actor in [Role::Client, Role::Provider] {
        for role in ROLES {
            let s = Setup::new();
            let kernel = s.kernel();
            let id = s.open_job(&kernel);
            let actor_address = address(&s, actor);
            sign_only_as(
                &s,
                role,
                &kernel.address,
                "set_budget",
                (id, actor_address.clone(), BUDGET).into_val(&s.env),
                &[],
            );

            let result = kernel.try_set_budget(&id, &actor_address, &BUDGET);

            assert_cell(
                result,
                role == actor,
                &format!("set_budget as {actor:?} signed by {role:?}"),
            );
            if role != actor {
                assert_eq!(kernel.get_job(&id).budget, 0);
            }
        }
    }
}

#[test]
fn only_the_client_can_fund() {
    for role in ROLES {
        let s = Setup::new();
        let kernel = s.kernel();
        let id = s.open_job(&kernel);
        kernel.set_budget(&id, &s.client, &BUDGET);
        let transfer = MockAuthInvoke {
            contract: &s.token.address,
            fn_name: "transfer",
            args: (s.client.clone(), kernel.address.clone(), BUDGET).into_val(&s.env),
            sub_invokes: &[],
        };
        sign_only_as(
            &s,
            role,
            &kernel.address,
            "fund",
            (id, BUDGET).into_val(&s.env),
            core::slice::from_ref(&transfer),
        );

        let result = kernel.try_fund(&id, &BUDGET);

        assert_cell(result, role == Role::Client, &format!("fund by {role:?}"));
        if role != Role::Client {
            assert_eq!(kernel.get_job(&id).state, Open);
            assert_eq!(s.balance(&kernel.address), 0);
        }
    }
}

#[test]
fn only_the_provider_can_submit() {
    for role in ROLES {
        let s = Setup::new();
        let kernel = s.kernel();
        let id = s.funded_job(&kernel);
        sign_only_as(
            &s,
            role,
            &kernel.address,
            "submit",
            (id, s.hash(2)).into_val(&s.env),
            &[],
        );

        let result = kernel.try_submit(&id, &s.hash(2));

        assert_cell(
            result,
            role == Role::Provider,
            &format!("submit by {role:?}"),
        );
        if role != Role::Provider {
            assert_eq!(kernel.get_job(&id).state, Funded);
        }
    }
}

#[test]
fn only_the_evaluator_can_complete() {
    for role in ROLES {
        let s = Setup::new();
        let kernel = s.kernel();
        let id = s.submitted_job(&kernel);
        sign_only_as(
            &s,
            role,
            &kernel.address,
            "complete",
            (id, s.hash(9)).into_val(&s.env),
            &[],
        );

        let result = kernel.try_complete(&id, &s.hash(9));

        assert_cell(
            result,
            role == Role::Evaluator,
            &format!("complete by {role:?}"),
        );
        if role != Role::Evaluator {
            assert_eq!(kernel.get_job(&id).state, Submitted);
            assert_eq!(s.balance(&s.provider), 0);
        }
    }
}

#[test]
fn reject_needs_the_client_while_open_and_the_evaluator_once_funded() {
    let cases: [(JobState, Role); 3] = [
        (Open, Role::Client),
        (Funded, Role::Evaluator),
        (Submitted, Role::Evaluator),
    ];
    for (state, rejector) in cases {
        for role in ROLES {
            let s = Setup::new();
            let kernel = s.kernel();
            let id = match state {
                Open => s.open_job(&kernel),
                Funded => s.funded_job(&kernel),
                _ => s.submitted_job(&kernel),
            };
            sign_only_as(
                &s,
                role,
                &kernel.address,
                "reject",
                (id, Option::<soroban_sdk::BytesN<32>>::None).into_val(&s.env),
                &[],
            );

            let result = kernel.try_reject(&id, &None);

            assert_cell(
                result,
                role == rejector,
                &format!("reject of {state:?} job by {role:?}"),
            );
            if role != rejector {
                assert_eq!(kernel.get_job(&id).state, state);
            }
        }
    }
}

#[test]
fn refunds_and_ttl_extension_need_no_signature_at_all() {
    // Enforcing mode with zero authorizations: if either function asked for
    // any signature, it would fail here.
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.funded_job(&kernel);
    s.advance_time(JOB_DURATION);
    s.env.mock_auths(&[]);

    assert!(kernel.try_extend_ttl(&id).is_ok());
    assert!(kernel.try_claim_refund(&id).is_ok());
    assert_eq!(kernel.get_job(&id).state, Expired);
}
