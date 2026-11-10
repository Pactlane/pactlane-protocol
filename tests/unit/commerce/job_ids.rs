//! A refused `create_job` must not consume an ID: IDs count accepted jobs
//! only, so `job_count` is also the highest ID and every ID up to it exists.

use pactlane_interfaces::{Error, MAX_JOB_DURATION};
use pactlane_tests::{Setup, JOB_DURATION};

#[test]
fn job_ids_stay_consecutive_across_refused_creations() {
    let s = Setup::new();
    let kernel = s.kernel();
    let later = s.now() + JOB_DURATION;
    let p = Some(s.provider.clone());

    let mut accepted = 0u64;
    for round in 0..6u32 {
        // One more refusal each round, cycling through every refusal kind.
        for attempt in 0..=round {
            let outcome = match (round + attempt) % 6 {
                0 => kernel.try_create_job(&s.client, &p, &s.evaluator, &s.now(), &s.hash(1)),
                1 => kernel.try_create_job(&s.client, &p, &s.evaluator, &(s.now() - 1), &s.hash(1)),
                2 => kernel.try_create_job(
                    &s.client,
                    &p,
                    &s.evaluator,
                    &(s.now() + MAX_JOB_DURATION + 1),
                    &s.hash(1),
                ),
                3 => kernel.try_create_job(&s.client, &p, &s.client, &later, &s.hash(1)),
                4 => kernel.try_create_job(
                    &s.client,
                    &Some(s.client.clone()),
                    &s.evaluator,
                    &later,
                    &s.hash(1),
                ),
                _ => kernel.try_create_job(
                    &s.client,
                    &Some(s.evaluator.clone()),
                    &s.evaluator,
                    &later,
                    &s.hash(1),
                ),
            };
            assert!(
                matches!(outcome, Err(Ok(Error::BadExpiry | Error::RoleConflict))),
                "round {round}, attempt {attempt}: {outcome:?}"
            );
            assert_eq!(kernel.job_count(), accepted);
            assert_eq!(
                kernel.try_get_job(&(accepted + 1)),
                Err(Ok(Error::NotFound))
            );
        }

        let id = s.open_job(&kernel);
        accepted += 1;
        assert_eq!(id, accepted, "round {round}");
        assert_eq!(kernel.job_count(), accepted);
    }

    for id in 1..=accepted {
        assert_eq!(kernel.get_job(&id).id, id);
    }
}
