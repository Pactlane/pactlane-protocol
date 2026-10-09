//! Every (state, action) pair, checked against the spec's transition table.
//!
//! ERC-8183 says "no other transitions are valid". Testing only the legal
//! paths would miss an illegal one that silently succeeds, such as completing
//! a job that was never submitted, so every cell is listed explicitly.

use pactlane_commerce::CommerceKernelClient;
use pactlane_interfaces::{Error, JobState, JobState::*};
use pactlane_tests::{Setup, BUDGET, JOB_DURATION};

#[derive(Clone, Copy, Debug)]
enum Action {
    SetProvider,
    SetBudget,
    Fund,
    Submit,
    Complete,
    Reject,
    ClaimRefund,
}

use Action::*;

const ACTIONS: [Action; 7] = [
    SetProvider,
    SetBudget,
    Fund,
    Submit,
    Complete,
    Reject,
    ClaimRefund,
];

/// What the spec says each action does in each state, for a live job that
/// already has a provider and a budget. `Ok(next)` is the resulting state.
fn expected(state: JobState, action: Action) -> Result<JobState, Error> {
    match (state, action) {
        (Open, SetProvider) => Err(Error::ProviderAlreadySet),
        (Open, SetBudget) => Ok(Open),
        (Open, Fund) => Ok(Funded),
        (Open, Reject) => Ok(Rejected),
        (Funded, Submit) => Ok(Submitted),
        (Funded, Reject) => Ok(Rejected),
        (Submitted, Complete) => Ok(Completed),
        (Submitted, Reject) => Ok(Rejected),
        // A live funded job's refund is refused for timing, not state.
        (Funded | Submitted, ClaimRefund) => Err(Error::NotExpired),
        _ => Err(Error::BadState),
    }
}

/// A job in `state`. Every job has a provider and a budget; all but
/// `Expired` are still live.
fn job_in(s: &Setup, kernel: &CommerceKernelClient, state: JobState) -> u64 {
    let id = match state {
        Open => {
            let id = s.open_job(kernel);
            kernel.set_budget(&id, &s.client, &BUDGET);
            id
        }
        Funded => s.funded_job(kernel),
        Submitted | Completed => s.submitted_job(kernel),
        Rejected | Expired => s.funded_job(kernel),
    };
    match state {
        Completed => kernel.complete(&id, &s.hash(9)),
        Rejected => kernel.reject(&id, &None),
        Expired => {
            s.advance_time(JOB_DURATION);
            kernel.claim_refund(&id);
        }
        _ => {}
    }
    assert_eq!(
        kernel.get_job(&id).state,
        state,
        "fixture built the wrong state"
    );
    id
}

fn act(s: &Setup, kernel: &CommerceKernelClient, id: u64, action: Action) -> Result<(), Error> {
    let result = match action {
        SetProvider => kernel.try_set_provider(&id, &s.stranger),
        SetBudget => kernel.try_set_budget(&id, &s.client, &BUDGET),
        Fund => kernel.try_fund(&id, &BUDGET),
        Submit => kernel.try_submit(&id, &s.hash(2)),
        Complete => kernel.try_complete(&id, &s.hash(9)),
        Reject => kernel.try_reject(&id, &None),
        ClaimRefund => kernel.try_claim_refund(&id),
    };
    match result {
        Ok(Ok(())) => Ok(()),
        Err(Ok(error)) => Err(error),
        other => panic!("{action:?} failed outside the contract: {other:?}"),
    }
}

#[test]
fn every_state_and_action_follows_the_transition_table() {
    for state in [Open, Funded, Submitted, Completed, Rejected, Expired] {
        for action in ACTIONS {
            let s = Setup::new();
            let kernel = s.kernel();
            let id = job_in(&s, &kernel, state);
            let before = kernel.get_job(&id);

            let outcome = act(&s, &kernel, id, action);

            let cell = format!("{action:?} on {state:?}");
            match expected(state, action) {
                Ok(next) => {
                    assert_eq!(outcome, Ok(()), "{cell}");
                    assert_eq!(kernel.get_job(&id).state, next, "{cell}");
                }
                Err(error) => {
                    assert_eq!(outcome, Err(error), "{cell}");
                    assert_eq!(kernel.get_job(&id), before, "{cell} changed the job");
                }
            }
        }
    }
}

#[test]
fn terminal_states_are_absorbing() {
    // T6: once a job pays out or refunds, nothing can act on it again. Stated
    // separately from the table so a regression names the hazard directly.
    for state in [Completed, Rejected, Expired] {
        let s = Setup::new();
        let kernel = s.kernel();
        let id = job_in(&s, &kernel, state);
        let escrow = s.balance(&kernel.address);

        for action in ACTIONS {
            assert_eq!(
                act(&s, &kernel, id, action),
                Err(Error::BadState),
                "{action:?} on {state:?}"
            );
        }
        assert_eq!(kernel.get_job(&id).state, state);
        assert_eq!(s.balance(&kernel.address), escrow);
    }
}
