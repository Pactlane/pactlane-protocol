//! Hashes are opaque commitments to the kernel: it never interprets them, so
//! an all-zero work hash is a valid value, stored and emitted verbatim.

use pactlane_interfaces::{JobState, JobSubmitted};
use pactlane_tests::{Setup, BUDGET};
use soroban_sdk::{BytesN, Event};

#[test]
fn an_all_zero_work_hash_is_accepted_and_stored_verbatim() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.funded_job(&kernel);
    let zero = BytesN::from_array(&s.env, &[0; 32]);

    kernel.submit(&id, &zero);

    assert_eq!(
        s.events_of(&kernel.address),
        [JobSubmitted {
            id,
            provider: s.provider.clone(),
            work_hash: zero.clone()
        }
        .to_xdr(&s.env, &kernel.address)]
    );
    let job = kernel.get_job(&id);
    assert_eq!(job.state, JobState::Submitted);
    assert_eq!(job.work_hash, Some(zero));

    // A zero commitment is still completable work.
    kernel.complete(&id, &s.hash(9));
    assert_eq!(s.balance(&s.provider), BUDGET);
}
