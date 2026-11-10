//! `settled` is declared with `sparse = false`: a rejection without a reason
//! still carries a `reason` key, holding void. Decoded from the XDR rather
//! than compared with the macro's own `to_xdr`.

use pactlane_tests::Setup;
use soroban_sdk::{
    testutils::Address as _,
    xdr::{ContractEventBody, ScSymbol, ScVal},
    Address,
};

#[test]
fn a_reject_with_no_reason_emits_settled_with_a_void_reason_key() {
    let s = Setup::new();
    let kernel = s.kernel();
    let policy = s.policy(&kernel.address, &Address::generate(&s.env));
    let signer = Address::generate(&s.env);
    policy.add_signer(&signer);
    let id = s.funded_job_evaluated_by(&kernel, &policy.address);

    policy.reject(&signer, &id, &None);

    let events = s.events_of(&policy.address);
    assert_eq!(events.events().len(), 1, "expected only settled");
    let ContractEventBody::V0(body) = &events.events()[0].body;
    assert_eq!(
        body.topics[0],
        ScVal::Symbol(ScSymbol("settled".try_into().unwrap()))
    );
    let ScVal::Map(Some(map)) = &body.data else {
        panic!("settled data is not a map: {:?}", body.data);
    };
    let entries: std::vec::Vec<(std::string::String, ScVal)> = map
        .iter()
        .map(|e| match &e.key {
            ScVal::Symbol(k) => (k.to_utf8_string_lossy(), e.val.clone()),
            other => panic!("non-symbol key {other:?}"),
        })
        .collect();
    assert_eq!(
        entries,
        [
            ("completed".into(), ScVal::Bool(false)),
            ("reason".into(), ScVal::Void),
        ]
    );
}
