//! Indexers filter kernel events by topic position: the event name first,
//! then the job ID (`specs/INTERFACES.md`, Events). The topics are decoded
//! from the emitted XDR and compared with literal names, so a renamed Rust
//! type or a reordered `#[topic]` field cannot slip through.

use pactlane_commerce::CommerceKernelClient;
use pactlane_tests::{Setup, BUDGET, JOB_DURATION};
use soroban_sdk::{
    xdr::{ContractEventBody, ScAddress, ScSymbol, ScVal},
    Address,
};

/// One event as emitted, with what the spec says its topics must be.
struct Emitted {
    name: &'static str,
    id: u64,
    /// The party the spec lists as the third topic, if any.
    party: Option<Address>,
    topics: std::vec::Vec<ScVal>,
}

fn symbol(name: &str) -> ScVal {
    ScVal::Symbol(ScSymbol(name.try_into().unwrap()))
}

/// Records the events of the call just made, which must be exactly
/// `expected` (name and third-topic party), in order.
fn take(
    s: &Setup,
    kernel: &CommerceKernelClient,
    id: u64,
    expected: &[(&'static str, Option<&Address>)],
    out: &mut std::vec::Vec<Emitted>,
) {
    let events = s.events_of(&kernel.address);
    assert_eq!(
        events.events().len(),
        expected.len(),
        "expected {expected:?} for job {id}"
    );
    for (event, (name, party)) in events.events().iter().zip(expected) {
        let ContractEventBody::V0(body) = &event.body;
        out.push(Emitted {
            name,
            id,
            party: party.cloned(),
            topics: body.topics.to_vec(),
        });
    }
}

/// Drives jobs through every path so that all ten kernel events are emitted.
fn all_ten_events(s: &Setup) -> std::vec::Vec<Emitted> {
    let kernel = s.kernel();
    let mut out = std::vec::Vec::new();
    // Throwaway jobs, so that no event's ID topic is 1 by coincidence.
    for _ in 0..3 {
        s.open_job(&kernel);
    }

    let paid = kernel.create_job(
        &s.client,
        &None,
        &s.evaluator,
        &(s.now() + JOB_DURATION),
        &s.hash(1),
    );
    take(
        s,
        &kernel,
        paid,
        &[("job_created", Some(&s.client))],
        &mut out,
    );
    kernel.set_provider(&paid, &s.provider);
    take(
        s,
        &kernel,
        paid,
        &[("provider_set", Some(&s.provider))],
        &mut out,
    );
    kernel.set_budget(&paid, &s.provider, &BUDGET);
    take(
        s,
        &kernel,
        paid,
        &[("budget_set", Some(&s.provider))],
        &mut out,
    );
    kernel.fund(&paid, &BUDGET);
    take(
        s,
        &kernel,
        paid,
        &[("job_funded", Some(&s.client))],
        &mut out,
    );
    kernel.submit(&paid, &s.hash(2));
    take(
        s,
        &kernel,
        paid,
        &[("job_submitted", Some(&s.provider))],
        &mut out,
    );
    kernel.complete(&paid, &s.hash(9));
    take(
        s,
        &kernel,
        paid,
        &[
            ("job_completed", Some(&s.evaluator)),
            ("payment_released", Some(&s.provider)),
        ],
        &mut out,
    );

    let rejected = s.funded_job(&kernel);
    kernel.reject(&rejected, &None);
    take(
        s,
        &kernel,
        rejected,
        &[
            ("job_rejected", Some(&s.evaluator)),
            ("refunded", Some(&s.client)),
        ],
        &mut out,
    );

    let expired = s.funded_job(&kernel);
    s.advance_time(JOB_DURATION);
    kernel.claim_refund(&expired);
    take(
        s,
        &kernel,
        expired,
        &[("job_expired", None), ("refunded", Some(&s.client))],
        &mut out,
    );

    out
}

#[test]
fn every_kernel_event_leads_with_its_name_then_the_job_id() {
    let s = Setup::new();

    let emitted = all_ten_events(&s);

    for e in &emitted {
        assert!(e.topics.len() >= 2, "{}: topics {:?}", e.name, e.topics);
        assert_eq!(e.topics[0], symbol(e.name), "first topic of {}", e.name);
        assert_eq!(e.topics[1], ScVal::U64(e.id), "second topic of {}", e.name);
    }
    let mut names: std::vec::Vec<_> = emitted.iter().map(|e| e.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(
        names,
        [
            "budget_set",
            "job_completed",
            "job_created",
            "job_expired",
            "job_funded",
            "job_rejected",
            "job_submitted",
            "payment_released",
            "provider_set",
            "refunded",
        ],
        "every one of the ten kernel events must be checked"
    );
}

#[test]
fn every_kernel_event_carries_exactly_the_topics_the_spec_lists() {
    // After the name and ID comes the acting or receiving party, except for
    // job_expired, which has none. Nothing else belongs in the topics.
    let s = Setup::new();

    for e in all_ten_events(&s) {
        let mut expected = std::vec![symbol(e.name), ScVal::U64(e.id)];
        if let Some(party) = &e.party {
            expected.push(ScVal::Address(ScAddress::from(party)));
        }
        assert_eq!(e.topics, expected, "topics of {}", e.name);
    }
}
