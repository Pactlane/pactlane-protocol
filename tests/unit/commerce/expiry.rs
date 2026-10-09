//! Behaviour at one second before, exactly at, and one second after
//! `expires_at`. Off-by-one errors here decide whether a provider is paid or
//! a client is refunded, so each boundary is pinned.

use pactlane_commerce::CommerceKernelClient;
use pactlane_interfaces::{Error, MAX_JOB_DURATION};
use pactlane_tests::{Setup, BUDGET, JOB_DURATION, START_TIME};

/// Jobs built by the fixture are created at `START_TIME`.
const EXPIRES_AT: u64 = START_TIME + JOB_DURATION;

/// The three instants around expiry, and whether the job is live at each.
const INSTANTS: [(i64, bool); 3] = [(-1, true), (0, false), (1, false)];

fn at(s: &Setup, offset: i64) {
    s.set_time(EXPIRES_AT.checked_add_signed(offset).unwrap());
}

fn outcome<T, C, H: core::fmt::Debug>(
    result: Result<Result<T, C>, Result<Error, H>>,
) -> Result<(), Error> {
    match result {
        Ok(Ok(_)) => Ok(()),
        Err(Ok(error)) => Err(error),
        Ok(Err(_)) => panic!("return value failed to convert"),
        Err(Err(host)) => panic!("failed outside the contract: {host:?}"),
    }
}

fn live_or_expired(live: bool) -> Result<(), Error> {
    if live {
        Ok(())
    } else {
        Err(Error::Expired)
    }
}

/// Runs `call` against a fresh job built by `build`, at each instant.
fn at_each_instant(
    name: &str,
    build: impl Fn(&Setup, &CommerceKernelClient) -> u64,
    call: impl Fn(&Setup, &CommerceKernelClient, u64) -> Result<(), Error>,
    expect: impl Fn(bool) -> Result<(), Error>,
) {
    for (offset, live) in INSTANTS {
        let s = Setup::new();
        let kernel = s.kernel();
        let id = build(&s, &kernel);
        at(&s, offset);

        assert_eq!(
            call(&s, &kernel, id),
            expect(live),
            "{name} at expires_at {offset:+}"
        );
    }
}

#[test]
fn open_job_actions_stop_at_expiry() {
    at_each_instant(
        "set_provider",
        |s, k| s.open_job_without_provider(k),
        |s, k, id| outcome(k.try_set_provider(&id, &s.provider)),
        live_or_expired,
    );
    at_each_instant(
        "set_budget",
        |s, k| s.open_job(k),
        |s, k, id| outcome(k.try_set_budget(&id, &s.client, &BUDGET)),
        live_or_expired,
    );
    at_each_instant(
        "fund",
        |s, k| {
            let id = s.open_job(k);
            k.set_budget(&id, &s.client, &BUDGET);
            id
        },
        |_, k, id| outcome(k.try_fund(&id, &BUDGET)),
        live_or_expired,
    );
}

#[test]
fn funded_job_actions_stop_at_expiry() {
    at_each_instant(
        "submit",
        |s, k| s.funded_job(k),
        |s, k, id| outcome(k.try_submit(&id, &s.hash(2))),
        live_or_expired,
    );
    at_each_instant(
        "complete",
        |s, k| s.submitted_job(k),
        |s, k, id| outcome(k.try_complete(&id, &s.hash(9))),
        live_or_expired,
    );
    at_each_instant(
        "evaluator reject",
        |s, k| s.submitted_job(k),
        |_, k, id| outcome(k.try_reject(&id, &None)),
        live_or_expired,
    );
}

#[test]
fn refunds_open_exactly_at_expiry() {
    at_each_instant(
        "claim_refund",
        |s, k| s.submitted_job(k),
        |_, k, id| outcome(k.try_claim_refund(&id)),
        |live| if live { Err(Error::NotExpired) } else { Ok(()) },
    );
}

#[test]
fn an_open_job_can_be_cancelled_at_any_instant() {
    at_each_instant(
        "client reject",
        |s, k| s.open_job(k),
        |_, k, id| outcome(k.try_reject(&id, &None)),
        |_| Ok(()),
    );
}

#[test]
fn at_every_instant_exactly_one_of_payout_or_refund_is_possible() {
    // T7: if both were possible at some instant, the evaluator and a refund
    // claimant could race for the same escrow; if neither were, it would be
    // stuck.
    for (offset, _) in INSTANTS {
        let payout = {
            let s = Setup::new();
            let kernel = s.kernel();
            let id = s.submitted_job(&kernel);
            at(&s, offset);
            kernel.try_complete(&id, &s.hash(9)).is_ok()
        };
        let refund = {
            let s = Setup::new();
            let kernel = s.kernel();
            let id = s.submitted_job(&kernel);
            at(&s, offset);
            kernel.try_claim_refund(&id).is_ok()
        };

        assert!(
            payout != refund,
            "at expires_at {offset:+}: payout possible = {payout}, refund possible = {refund}"
        );
    }
}

#[test]
fn creation_accepts_expiries_from_one_second_to_the_maximum() {
    let s = Setup::new();
    let kernel = s.kernel();

    for (expires_at, accepted) in [
        (s.now(), false),
        (s.now() + 1, true),
        (s.now() + MAX_JOB_DURATION, true),
        (s.now() + MAX_JOB_DURATION + 1, false),
    ] {
        let result = kernel.try_create_job(&s.client, &None, &s.evaluator, &expires_at, &s.hash(1));
        assert_eq!(
            result.is_ok(),
            accepted,
            "expires_at = now + {}",
            expires_at - s.now()
        );
        if !accepted {
            assert_eq!(result, Err(Ok(Error::BadExpiry)));
        }
    }
}
