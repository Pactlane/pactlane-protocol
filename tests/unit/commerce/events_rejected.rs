//! `job_rejected` is declared with `sparse = false`, so its data map always
//! has a `reason` key, holding void when no reason was given.
//!
//! The emitted XDR is decoded by hand rather than compared with
//! `JobRejected { .. }.to_xdr(..)`, which comes from the same macro as the
//! event and would hide a change in its encoding.

use pactlane_tests::Setup;
use soroban_sdk::{
    xdr::{ContractEvent, ContractEventBody, ScSymbol, ScVal},
    BytesN, FromVal, IntoVal, Val,
};

/// The event's first topic, its name.
fn name(event: &ContractEvent) -> ScVal {
    let ContractEventBody::V0(body) = &event.body;
    body.topics[0].clone()
}

/// The event's data map as (key, value) pairs, in the order emitted.
fn data_entries(event: &ContractEvent) -> std::vec::Vec<(std::string::String, ScVal)> {
    let ContractEventBody::V0(body) = &event.body;
    let ScVal::Map(Some(map)) = &body.data else {
        panic!("event data is not a map: {:?}", body.data);
    };
    map.iter()
        .map(|entry| match &entry.key {
            ScVal::Symbol(key) => (key.to_utf8_string_lossy(), entry.val.clone()),
            other => panic!("event data key is not a symbol: {other:?}"),
        })
        .collect()
}

fn job_rejected() -> ScVal {
    ScVal::Symbol(ScSymbol("job_rejected".try_into().unwrap()))
}

/// The data of the single `job_rejected` event among `events`.
fn rejected_data(events: &[ContractEvent]) -> std::vec::Vec<(std::string::String, ScVal)> {
    let mut rejected = events.iter().filter(|e| name(e) == job_rejected());
    let event = rejected.next().expect("no job_rejected event");
    assert!(
        rejected.next().is_none(),
        "more than one job_rejected event"
    );
    data_entries(event)
}

#[test]
fn a_cancellation_without_a_reason_still_carries_a_void_reason_key() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job(&kernel);

    kernel.reject(&id, &None);

    let events = s.events_of(&kernel.address);
    assert_eq!(
        events.events().len(),
        1,
        "a cancellation moves no funds, so it emits only job_rejected"
    );
    assert_eq!(
        rejected_data(events.events()),
        [("reason".into(), ScVal::Void)],
        "a missing key would break indexers that read the map by name"
    );
}

#[test]
fn an_evaluator_rejection_without_a_reason_still_carries_a_void_reason_key() {
    // The other way a job is rejected: the refund event alongside it must
    // not change the shape of job_rejected.
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.submitted_job(&kernel);

    kernel.reject(&id, &None);

    let events = s.events_of(&kernel.address);
    assert_eq!(
        rejected_data(events.events()),
        [("reason".into(), ScVal::Void)]
    );
}

#[test]
fn a_rejection_with_a_reason_carries_the_hash_under_the_same_key() {
    // The counterpart of the void cases: proves the key is read correctly.
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job(&kernel);
    let reason: BytesN<32> = s.hash(8);
    let reason_val: Val = reason.clone().into_val(&s.env);

    kernel.reject(&id, &Some(reason));

    let events = s.events_of(&kernel.address);
    assert_eq!(
        rejected_data(events.events()),
        [("reason".into(), ScVal::from_val(&s.env, &reason_val))]
    );
}
