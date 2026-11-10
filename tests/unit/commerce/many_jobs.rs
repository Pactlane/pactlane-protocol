//! Fifty jobs between the same client, provider and evaluator, settled with
//! every outcome. Each job has a distinct budget, so a payout or refund that
//! moved another job's amount, or moved one twice, shows in the totals.

use pactlane_interfaces::JobState;
use pactlane_tests::{Setup, JOB_DURATION, STARTING_BALANCE, USDC};

const JOBS: u64 = 50;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Outcome {
    /// Submitted and completed: the provider is paid.
    Paid,
    /// Rejected by the evaluator while funded: the client is refunded.
    RejectedFunded,
    /// Rejected by the evaluator after submission: the client is refunded.
    RejectedSubmitted,
    /// Left funded until expiry, then refunded with `claim_refund`.
    Expired,
    /// Cancelled by the client while open: nothing ever moved.
    Cancelled,
}

use Outcome::*;

impl Outcome {
    fn for_job(index: u64) -> Self {
        [Paid, RejectedFunded, RejectedSubmitted, Expired, Cancelled][(index % 5) as usize]
    }

    fn final_state(self) -> JobState {
        match self {
            Paid => JobState::Completed,
            RejectedFunded | RejectedSubmitted | Cancelled => JobState::Rejected,
            Expired => JobState::Expired,
        }
    }
}

/// A distinct budget per job: one token plus the job's index in base units.
fn budget_of(index: u64) -> i128 {
    USDC + i128::from(index)
}

#[test]
fn fifty_jobs_between_the_same_parties_settle_to_exactly_their_own_outcomes() {
    // Fifty job entries make a ~450 KB snapshot, too large to review as a
    // diff; the balance assertions below are what protect this behaviour.
    let s = Setup::without_snapshots();
    let kernel = s.kernel();
    let jobs: std::vec::Vec<(u64, Outcome, i128)> = (0..JOBS)
        .map(|index| {
            let id = s.open_job(&kernel);
            let budget = budget_of(index);
            kernel.set_budget(&id, &s.client, &budget);
            (id, Outcome::for_job(index), budget)
        })
        .collect();
    let total = |pick: &dyn Fn(Outcome) -> bool| -> i128 {
        jobs.iter()
            .filter(|(_, outcome, _)| pick(*outcome))
            .map(|(_, _, budget)| budget)
            .sum()
    };
    let paid = total(&|o| o == Paid);
    let funded = total(&|o| o != Cancelled);
    let expired = total(&|o| o == Expired);

    for &(id, outcome, budget) in &jobs {
        if outcome != Cancelled {
            kernel.fund(&id, &budget);
        }
        if matches!(outcome, Paid | RejectedSubmitted) {
            kernel.submit(&id, &s.hash(2));
        }
    }
    assert_eq!(s.balance(&kernel.address), funded);
    assert_eq!(s.balance(&s.client), STARTING_BALANCE - funded);

    // Settled newest first, so no outcome depends on creation order.
    for &(id, outcome, _) in jobs.iter().rev() {
        match outcome {
            Paid => kernel.complete(&id, &s.hash(9)),
            RejectedFunded | RejectedSubmitted | Cancelled => kernel.reject(&id, &None),
            Expired => {}
        }
    }
    assert_eq!(s.balance(&kernel.address), expired);

    s.advance_time(JOB_DURATION);
    for &(id, outcome, _) in &jobs {
        if outcome == Expired {
            kernel.claim_refund(&id);
        }
    }

    for &(id, outcome, budget) in &jobs {
        let job = kernel.get_job(&id);
        assert_eq!(job.state, outcome.final_state(), "job {id} ({outcome:?})");
        assert_eq!(job.budget, budget, "job {id} ({outcome:?})");
    }
    assert_eq!(kernel.job_count(), JOBS);
    assert_eq!(s.balance(&s.provider), paid);
    assert_eq!(s.balance(&s.client), STARTING_BALANCE - paid);
    assert_eq!(s.balance(&kernel.address), 0);
    assert_eq!(s.balance(&s.evaluator), 0);
    assert_eq!(s.balance(&s.stranger), 0);
}
