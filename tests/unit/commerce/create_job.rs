use pactlane_interfaces::{Error, Job, JobCreated, JobState, MAX_JOB_DURATION};
use pactlane_tests::{Setup, JOB_DURATION, START_TIME};
use soroban_sdk::Event;

#[test]
fn a_new_job_is_open_with_no_budget_and_the_agreed_terms() {
    let s = Setup::new();
    let kernel = s.kernel();

    let id = s.open_job(&kernel);

    assert_eq!(
        kernel.get_job(&id),
        Job {
            id,
            client: s.client.clone(),
            provider: Some(s.provider.clone()),
            evaluator: s.evaluator.clone(),
            spec_hash: s.hash(1),
            budget: 0,
            expires_at: START_TIME + JOB_DURATION,
            state: JobState::Open,
            work_hash: None,
            reason: None,
        }
    );
}

#[test]
fn job_ids_start_at_one_and_count_up() {
    let s = Setup::new();
    let kernel = s.kernel();

    assert_eq!(s.open_job(&kernel), 1);
    assert_eq!(s.open_job(&kernel), 2);
    assert_eq!(kernel.job_count(), 2);
}

#[test]
fn only_the_client_authorizes_creation() {
    let s = Setup::new();
    let kernel = s.kernel();

    s.open_job(&kernel);

    s.assert_authorized_by(&s.client, &kernel.address, "create_job");
}

#[test]
fn creation_emits_job_created_with_the_terms() {
    let s = Setup::new();
    let kernel = s.kernel();

    let id = s.open_job(&kernel);

    assert_eq!(
        s.events_of(&kernel.address),
        [JobCreated {
            id,
            client: s.client.clone(),
            provider: Some(s.provider.clone()),
            evaluator: s.evaluator.clone(),
            expires_at: START_TIME + JOB_DURATION,
            spec_hash: s.hash(1),
        }
        .to_xdr(&s.env, &kernel.address)]
    );
}

#[test]
fn a_job_can_be_opened_before_a_provider_is_chosen() {
    let s = Setup::new();
    let kernel = s.kernel();

    let id = kernel.create_job(
        &s.client,
        &None,
        &s.evaluator,
        &(s.now() + JOB_DURATION),
        &s.hash(1),
    );

    assert_eq!(kernel.get_job(&id).provider, None);
}

#[test]
fn an_expiry_that_is_not_in_the_future_is_refused() {
    let s = Setup::new();
    let kernel = s.kernel();

    for expires_at in [s.now() - 1, s.now()] {
        assert_eq!(
            kernel.try_create_job(&s.client, &None, &s.evaluator, &expires_at, &s.hash(1)),
            Err(Ok(Error::BadExpiry)),
            "expires_at = now {:+}",
            expires_at as i64 - s.now() as i64
        );
    }
}

#[test]
fn a_job_may_last_exactly_the_maximum_duration_and_no_longer() {
    let s = Setup::new();
    let kernel = s.kernel();
    let limit = s.now() + MAX_JOB_DURATION;

    assert_eq!(
        kernel.try_create_job(&s.client, &None, &s.evaluator, &(limit + 1), &s.hash(1)),
        Err(Ok(Error::BadExpiry))
    );
    assert!(kernel
        .try_create_job(&s.client, &None, &s.evaluator, &limit, &s.hash(1))
        .is_ok());
}

#[test]
fn the_client_cannot_evaluate_its_own_job() {
    // T2: a client-evaluator could reject finished work and keep its money.
    let s = Setup::new();
    let kernel = s.kernel();

    assert_eq!(
        kernel.try_create_job(
            &s.client,
            &Some(s.provider.clone()),
            &s.client,
            &(s.now() + JOB_DURATION),
            &s.hash(1)
        ),
        Err(Ok(Error::RoleConflict))
    );
}

#[test]
fn the_provider_cannot_evaluate_its_own_work() {
    // T1: a provider-evaluator could approve its own work unconditionally.
    let s = Setup::new();
    let kernel = s.kernel();

    assert_eq!(
        kernel.try_create_job(
            &s.client,
            &Some(s.evaluator.clone()),
            &s.evaluator,
            &(s.now() + JOB_DURATION),
            &s.hash(1)
        ),
        Err(Ok(Error::RoleConflict))
    );
}

#[test]
fn the_client_cannot_be_its_own_provider() {
    // T3: paying yourself fabricates work history.
    let s = Setup::new();
    let kernel = s.kernel();

    assert_eq!(
        kernel.try_create_job(
            &s.client,
            &Some(s.client.clone()),
            &s.evaluator,
            &(s.now() + JOB_DURATION),
            &s.hash(1)
        ),
        Err(Ok(Error::RoleConflict))
    );
}

#[test]
fn a_refused_creation_leaves_no_job_behind() {
    let s = Setup::new();
    let kernel = s.kernel();

    let _ = kernel.try_create_job(&s.client, &None, &s.client, &(s.now() + 1), &s.hash(1));

    assert_eq!(kernel.job_count(), 0);
    assert_eq!(kernel.try_get_job(&1), Err(Ok(Error::NotFound)));
}

#[test]
fn an_unknown_job_is_not_found() {
    let s = Setup::new();
    let kernel = s.kernel();

    assert_eq!(kernel.try_get_job(&0), Err(Ok(Error::NotFound)));
    assert_eq!(kernel.try_get_job(&1), Err(Ok(Error::NotFound)));
}
