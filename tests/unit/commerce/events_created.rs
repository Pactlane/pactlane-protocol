//! `job_created` is declared with `sparse = false`, so an indexer always finds
//! every key in its data map, with void standing in for `None`.
//!
//! These tests decode the emitted XDR by hand instead of comparing it with
//! `JobCreated { .. }.to_xdr(..)`: that value comes from the same macro as the
//! event itself, so a change to the macro's encoding would pass unnoticed.

use pactlane_tests::Setup;
use soroban_sdk::{
    xdr::{ContractEvent, ContractEventBody, ScVal},
    FromVal, IntoVal, Val,
};

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

fn keys(entries: &[(std::string::String, ScVal)]) -> std::vec::Vec<&str> {
    entries.iter().map(|(key, _)| key.as_str()).collect()
}

fn value<'a>(entries: &'a [(std::string::String, ScVal)], key: &str) -> &'a ScVal {
    &entries
        .iter()
        .find(|(k, _)| k == key)
        .unwrap_or_else(|| panic!("no `{key}` key in {entries:?}"))
        .1
}

fn sc_val(s: &Setup, value: impl IntoVal<soroban_sdk::Env, Val>) -> ScVal {
    ScVal::from_val(&s.env, &value.into_val(&s.env))
}

#[test]
fn a_job_created_without_a_provider_still_carries_a_void_provider_key() {
    let s = Setup::new();
    let kernel = s.kernel();

    s.open_job_without_provider(&kernel);

    let events = s.events_of(&kernel.address);
    assert_eq!(events.events().len(), 1, "expected only job_created");
    let data = data_entries(&events.events()[0]);
    assert_eq!(
        keys(&data),
        ["evaluator", "expires_at", "provider", "spec_hash"],
        "a missing key would break indexers that read the map by name"
    );
    assert_eq!(value(&data, "provider"), &ScVal::Void);
    // The void provider must not displace the fields around it.
    assert_eq!(value(&data, "evaluator"), &sc_val(&s, s.evaluator.clone()));
}

#[test]
fn a_job_created_with_a_provider_carries_its_address_under_the_same_key() {
    // The counterpart of the void case: proves the key is read correctly,
    // so the test above cannot pass by decoding the wrong field.
    let s = Setup::new();
    let kernel = s.kernel();

    s.open_job(&kernel);

    let events = s.events_of(&kernel.address);
    let data = data_entries(&events.events()[0]);
    assert_eq!(
        keys(&data),
        ["evaluator", "expires_at", "provider", "spec_hash"]
    );
    assert_eq!(value(&data, "provider"), &sc_val(&s, s.provider.clone()));
}
