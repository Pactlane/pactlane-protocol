//! Random sequences of job operations, with every invariant checked after
//! every step (specs/INVARIANTS.md, invariants 1–3, 7 and 8).
//!
//! No reference model is needed. After each step the test reads every job
//! back and checks that each party's balance is exactly what the jobs' states
//! say it should be. A double payout, a missing refund or a payment to the
//! wrong party breaks one of these identities.

use std::collections::BTreeMap;

use pactlane_commerce::CommerceKernelClient;
use pactlane_interfaces::{Job, JobState};
use pactlane_tests::{Setup, JOB_DURATION, STARTING_BALANCE, USDC};
use proptest::prelude::*;

#[derive(Clone, Debug)]
enum Op {
    Create {
        with_provider: bool,
        duration: u64,
    },
    SetProvider {
        job: usize,
    },
    SetBudget {
        job: usize,
        by_provider: bool,
        amount: i128,
    },
    /// Funds with `expected_budget = stored budget + skew`.
    Fund {
        job: usize,
        skew: i128,
    },
    Submit {
        job: usize,
    },
    Complete {
        job: usize,
    },
    Reject {
        job: usize,
    },
    ClaimRefund {
        job: usize,
    },
    Advance {
        seconds: u64,
    },
}

fn op() -> impl Strategy<Value = Op> {
    let job = 0..8usize;
    let amount = prop_oneof![
        8 => 1..=50 * USDC,
        1 => Just(STARTING_BALANCE + 1), // more than the client holds
        1 => -2..=0i128,                 // never valid
    ];
    prop_oneof![
        3 => (any::<bool>(), 1..=2 * JOB_DURATION)
            .prop_map(|(with_provider, duration)| Op::Create { with_provider, duration }),
        1 => job.clone().prop_map(|job| Op::SetProvider { job }),
        3 => (job.clone(), any::<bool>(), amount)
            .prop_map(|(job, by_provider, amount)| Op::SetBudget { job, by_provider, amount }),
        3 => (job.clone(), prop_oneof![6 => Just(0i128), 1 => -1..=1i128])
            .prop_map(|(job, skew)| Op::Fund { job, skew }),
        2 => job.clone().prop_map(|job| Op::Submit { job }),
        2 => job.clone().prop_map(|job| Op::Complete { job }),
        2 => job.clone().prop_map(|job| Op::Reject { job }),
        2 => job.prop_map(|job| Op::ClaimRefund { job }),
        2 => prop_oneof![0..=3_600u64, 0..=JOB_DURATION].prop_map(|seconds| Op::Advance { seconds }),
    ]
}

/// Maps an arbitrary index onto an existing job ID, if any job exists.
fn pick(kernel: &CommerceKernelClient, index: usize) -> Option<u64> {
    let count = kernel.job_count();
    (count > 0).then(|| index as u64 % count + 1)
}

/// Applies `op`, ignoring whether the kernel accepted it: refused operations
/// must leave the invariants intact just as accepted ones must. Returns the
/// budget a successful `fund` was told to expect.
fn apply(s: &Setup, kernel: &CommerceKernelClient, op: &Op) -> Option<(u64, i128)> {
    match *op {
        Op::Create {
            with_provider,
            duration,
        } => {
            let provider = with_provider.then(|| s.provider.clone());
            let _ = kernel.try_create_job(
                &s.client,
                &provider,
                &s.evaluator,
                &(s.now() + duration),
                &s.hash(1),
            );
        }
        Op::SetProvider { job } => {
            if let Some(id) = pick(kernel, job) {
                let _ = kernel.try_set_provider(&id, &s.provider);
            }
        }
        Op::SetBudget {
            job,
            by_provider,
            amount,
        } => {
            if let Some(id) = pick(kernel, job) {
                let actor = if by_provider { &s.provider } else { &s.client };
                let _ = kernel.try_set_budget(&id, actor, &amount);
            }
        }
        Op::Fund { job, skew } => {
            if let Some(id) = pick(kernel, job) {
                let expected = kernel.get_job(&id).budget + skew;
                if kernel.try_fund(&id, &expected).is_ok() {
                    return Some((id, expected));
                }
            }
        }
        Op::Submit { job } => {
            if let Some(id) = pick(kernel, job) {
                let _ = kernel.try_submit(&id, &s.hash(2));
            }
        }
        Op::Complete { job } => {
            if let Some(id) = pick(kernel, job) {
                let _ = kernel.try_complete(&id, &s.hash(9));
            }
        }
        Op::Reject { job } => {
            if let Some(id) = pick(kernel, job) {
                let _ = kernel.try_reject(&id, &None);
            }
        }
        Op::ClaimRefund { job } => {
            if let Some(id) = pick(kernel, job) {
                let _ = kernel.try_claim_refund(&id);
            }
        }
        Op::Advance { seconds } => s.advance_time(seconds),
    }
    None
}

fn all_jobs(kernel: &CommerceKernelClient) -> BTreeMap<u64, Job> {
    (1..=kernel.job_count())
        .map(|id| (id, kernel.get_job(&id)))
        .collect()
}

fn sum(jobs: &BTreeMap<u64, Job>, states: &[JobState]) -> i128 {
    jobs.values()
        .filter(|job| states.contains(&job.state))
        .map(|job| job.budget)
        .sum()
}

fn check_invariants(
    s: &Setup,
    kernel: &CommerceKernelClient,
    before: &BTreeMap<u64, Job>,
    funded: Option<(u64, i128)>,
) -> Result<BTreeMap<u64, Job>, TestCaseError> {
    let jobs = all_jobs(kernel);
    let escrowed = sum(&jobs, &[JobState::Funded, JobState::Submitted]);
    let paid = sum(&jobs, &[JobState::Completed]);

    // 1. The kernel holds exactly the budgets of jobs that hold funds.
    prop_assert_eq!(
        s.balance(&kernel.address),
        escrowed,
        "escrow != live budgets"
    );
    // 2. Completed jobs paid their provider exactly their budget, once.
    prop_assert_eq!(
        s.balance(&s.provider),
        paid,
        "provider paid != completed budgets"
    );
    // 3. The client paid exactly for jobs that are funded or completed; every
    //    rejected or expired funded job came back in full.
    prop_assert_eq!(
        s.balance(&s.client),
        STARTING_BALANCE - escrowed - paid,
        "client balance != what its jobs imply"
    );
    // Nobody else ever receives anything, and no tokens appear or vanish.
    prop_assert_eq!(s.balance(&s.evaluator), 0);
    prop_assert_eq!(s.balance(&s.stranger), 0);

    for (id, old) in before {
        let new = &jobs[id];
        // Terminal states are absorbing.
        if old.state.is_terminal() {
            prop_assert_eq!(new, old, "terminal job {} changed", id);
        }
        // Once funded, the budget never changes.
        if old.state != JobState::Open {
            prop_assert_eq!(
                new.budget,
                old.budget,
                "budget of job {} changed after funding",
                id
            );
        }
        // 8. Settlement follows the published expiry rule: a funded job is
        //    refunded by expiry only once it has expired, and is paid,
        //    submitted or rejected by its evaluator only while it is live.
        if new.state != old.state && old.state != JobState::Open {
            let expired = s.now() >= new.expires_at;
            let by_expiry = new.state == JobState::Expired;
            prop_assert_eq!(
                expired,
                by_expiry,
                "job {} went {:?} -> {:?} at expires_at {:+}",
                id,
                old.state,
                new.state,
                s.now() as i64 - new.expires_at as i64
            );
        }
    }
    // 7. A successful fund always matched the stored budget exactly.
    if let Some((id, expected)) = funded {
        prop_assert_eq!(
            before[&id].budget,
            expected,
            "job {} funded at the wrong amount",
            id
        );
    }
    Ok(jobs)
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: std::env::var("PROPTEST_CASES").ok().and_then(|n| n.parse().ok()).unwrap_or(64),
        ..ProptestConfig::default()
    })]

    #[test]
    fn money_is_conserved_under_any_sequence_of_operations(ops in prop::collection::vec(op(), 1..60)) {
        let s = Setup::without_snapshots();
        let kernel = s.kernel();
        let mut jobs = all_jobs(&kernel);

        for op in &ops {
            let funded = apply(&s, &kernel, op);
            jobs = check_invariants(&s, &kernel, &jobs, funded)?;
        }
    }
}
