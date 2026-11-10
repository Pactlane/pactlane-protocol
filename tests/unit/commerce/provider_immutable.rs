//! A provider named at creation is fixed for the job's life. In every
//! non-terminal state `set_provider` refuses to replace it. Which error is
//! reported follows the spec's check order (state, then liveness, then
//! `ProviderAlreadySet`), so only a live Open job reports
//! `ProviderAlreadySet`; the others report `BadState` or `Expired` first.

use pactlane_interfaces::{Error, JobState};
use pactlane_tests::{Setup, JOB_DURATION};

#[test]
fn a_provider_named_at_creation_cannot_be_replaced_in_any_live_state() {
    let s = Setup::new();
    let kernel = s.kernel();
    let open = s.open_job(&kernel);
    let funded = s.funded_job(&kernel);
    let submitted = s.submitted_job(&kernel);

    for (id, state, error) in [
        (open, JobState::Open, Error::ProviderAlreadySet),
        (funded, JobState::Funded, Error::BadState),
        (submitted, JobState::Submitted, Error::BadState),
    ] {
        assert_eq!(kernel.get_job(&id).state, state);
        for replacement in [&s.stranger, &s.evaluator, &s.client, &s.provider] {
            assert_eq!(
                kernel.try_set_provider(&id, replacement),
                Err(Ok(error)),
                "{state:?}, replacement {replacement:?}"
            );
        }
        assert_eq!(kernel.get_job(&id).provider, Some(s.provider.clone()));
    }
}

#[test]
fn a_provider_named_at_creation_cannot_be_replaced_after_expiry_either() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job(&kernel);
    s.advance_time(JOB_DURATION);

    assert_eq!(
        kernel.try_set_provider(&id, &s.stranger),
        Err(Ok(Error::Expired))
    );
    assert_eq!(kernel.get_job(&id).provider, Some(s.provider.clone()));
}
