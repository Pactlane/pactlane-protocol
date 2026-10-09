use pactlane_interfaces::{Error, ProviderSet};
use pactlane_tests::{Setup, JOB_DURATION};
use soroban_sdk::Event;

#[test]
fn the_client_assigns_a_provider_to_an_open_job() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job_without_provider(&kernel);

    kernel.set_provider(&id, &s.provider);

    s.assert_authorized_by(&s.client, &kernel.address, "set_provider");
    assert_eq!(
        s.events_of(&kernel.address),
        [ProviderSet {
            id,
            provider: s.provider.clone()
        }
        .to_xdr(&s.env, &kernel.address)]
    );
    assert_eq!(kernel.get_job(&id).provider, Some(s.provider.clone()));
}

#[test]
fn a_provider_is_assigned_only_once() {
    // Swapping the provider after it agreed would let the client hand the
    // job to someone else mid-negotiation.
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job(&kernel);

    assert_eq!(
        kernel.try_set_provider(&id, &s.stranger),
        Err(Ok(Error::ProviderAlreadySet))
    );
    assert_eq!(kernel.get_job(&id).provider, Some(s.provider.clone()));
}

#[test]
fn the_assigned_provider_cannot_hold_another_role() {
    // T1 and T3, through set_provider rather than create_job.
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job_without_provider(&kernel);

    for (who, name) in [(&s.client, "client"), (&s.evaluator, "evaluator")] {
        assert_eq!(
            kernel.try_set_provider(&id, who),
            Err(Ok(Error::RoleConflict)),
            "{name} as provider"
        );
    }
    assert_eq!(kernel.get_job(&id).provider, None);
}

#[test]
fn a_provider_cannot_be_assigned_once_the_job_expires() {
    let s = Setup::new();
    let kernel = s.kernel();
    let id = s.open_job_without_provider(&kernel);

    s.advance_time(JOB_DURATION);

    assert_eq!(
        kernel.try_set_provider(&id, &s.provider),
        Err(Ok(Error::Expired))
    );
}

#[test]
fn assigning_a_provider_to_an_unknown_job_fails() {
    let s = Setup::new();
    let kernel = s.kernel();

    assert_eq!(
        kernel.try_set_provider(&1, &s.provider),
        Err(Ok(Error::NotFound))
    );
}
