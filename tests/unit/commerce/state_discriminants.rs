//! `JobState` crosses the ABI as a `u32`, and generated bindings map the
//! number to a name. Renumbering a variant would silently relabel every
//! job, so the values are pinned, both in Rust and as encoded on chain.

use pactlane_interfaces::JobState;
use pactlane_tests::Setup;
use soroban_sdk::{xdr::ScVal, FromVal, IntoVal, Val};

const STATES: [(JobState, u32); 6] = [
    (JobState::Open, 0),
    (JobState::Funded, 1),
    (JobState::Submitted, 2),
    (JobState::Completed, 3),
    (JobState::Rejected, 4),
    (JobState::Expired, 5),
];

#[test]
fn job_state_discriminants_are_open_zero_through_expired_five() {
    for (state, number) in STATES {
        assert_eq!(state as u32, number, "{state:?}");
    }
}

#[test]
fn job_states_are_encoded_on_chain_as_their_numbers() {
    let s = Setup::new();
    for (state, number) in STATES {
        let val: Val = state.into_val(&s.env);
        assert_eq!(
            ScVal::from_val(&s.env, &val),
            ScVal::U32(number),
            "{state:?}"
        );
    }
}
