//! Differential test: a plain-Rust model of the kernel's state machine,
//! written from `specs/INTERFACES.md` (checks in the listed order, first
//! failure wins), runs beside the contract. After every operation of a
//! random sequence the model's predicted outcome, every job record, and
//! every balance must equal the contract's.

use pactlane_commerce::CommerceKernelClient;
use pactlane_interfaces::{Error, Job, JobState, MAX_JOB_DURATION};
use pactlane_tests::{Setup, STARTING_BALANCE};
use proptest::prelude::*;

use crate::conservation::{all_jobs, op, Op};

/// What a call did: succeeded, or was refused with a contract error code,
/// decoded with the kernel's error table.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Outcome {
    Ok,
    Refused(Error),
    /// Failed with something that is not a contract error code at all.
    Other,
}

/// The model's state: jobs, the clock, and the three balances.
struct Model {
    jobs: std::vec::Vec<Job>,
    client: i128,
    provider: i128,
    escrow: i128,
}

impl Model {
    fn new() -> Self {
        Self {
            jobs: std::vec::Vec::new(),
            client: STARTING_BALANCE,
            provider: 0,
            escrow: 0,
        }
    }

    /// The job an arbitrary index maps onto, as `pick` does for the contract.
    fn pick(&self, index: usize) -> Option<usize> {
        (!self.jobs.is_empty()).then(|| index % self.jobs.len())
    }

    fn step(&mut self, s: &Setup, op: &Op) -> Option<Outcome> {
        let now = s.now();
        let live = |job: &Job| now < job.expires_at;
        let outcome = match *op {
            Op::Create {
                with_provider,
                duration,
            } => {
                if duration > MAX_JOB_DURATION {
                    Outcome::Refused(Error::BadExpiry)
                } else {
                    let id = self.jobs.len() as u64 + 1;
                    self.jobs.push(Job {
                        id,
                        client: s.client.clone(),
                        provider: with_provider.then(|| s.provider.clone()),
                        evaluator: s.evaluator.clone(),
                        spec_hash: s.hash(1),
                        budget: 0,
                        expires_at: now + duration,
                        state: JobState::Open,
                        work_hash: None,
                        reason: None,
                    });
                    Outcome::Ok
                }
            }
            Op::SetProvider { job } => {
                let i = self.pick(job)?;
                let j = &mut self.jobs[i];
                if j.state != JobState::Open {
                    Outcome::Refused(Error::BadState)
                } else if !live(j) {
                    Outcome::Refused(Error::Expired)
                } else if j.provider.is_some() {
                    Outcome::Refused(Error::ProviderAlreadySet)
                } else {
                    j.provider = Some(s.provider.clone());
                    Outcome::Ok
                }
            }
            Op::SetBudget {
                job,
                by_provider,
                amount,
            } => {
                let i = self.pick(job)?;
                let j = &mut self.jobs[i];
                if by_provider && j.provider.is_none() {
                    Outcome::Refused(Error::BadActor)
                } else if j.state != JobState::Open {
                    Outcome::Refused(Error::BadState)
                } else if !live(j) {
                    Outcome::Refused(Error::Expired)
                } else if amount <= 0 {
                    Outcome::Refused(Error::BadBudget)
                } else {
                    j.budget = amount;
                    Outcome::Ok
                }
            }
            Op::Fund { job, skew } => {
                let i = self.pick(job)?;
                let j = &mut self.jobs[i];
                if j.state != JobState::Open {
                    Outcome::Refused(Error::BadState)
                } else if !live(j) {
                    Outcome::Refused(Error::Expired)
                } else if j.provider.is_none() {
                    Outcome::Refused(Error::NoProvider)
                } else if j.budget <= 0 {
                    Outcome::Refused(Error::BadBudget)
                } else if skew != 0 {
                    Outcome::Refused(Error::BudgetMismatch)
                } else if self.client < j.budget {
                    // The token's own error passes through: the Stellar Asset
                    // Contract's insufficient-balance error is code 10, which
                    // reads as the kernel's ProviderAlreadySet. See the note
                    // under Errors in specs/INTERFACES.md.
                    Outcome::Refused(Error::ProviderAlreadySet)
                } else {
                    j.state = JobState::Funded;
                    self.client -= j.budget;
                    self.escrow += j.budget;
                    Outcome::Ok
                }
            }
            Op::Submit { job } => {
                let i = self.pick(job)?;
                let j = &mut self.jobs[i];
                if j.provider.is_none() || j.state != JobState::Funded {
                    Outcome::Refused(Error::BadState)
                } else if !live(j) {
                    Outcome::Refused(Error::Expired)
                } else {
                    j.work_hash = Some(s.hash(2));
                    j.state = JobState::Submitted;
                    Outcome::Ok
                }
            }
            Op::Complete { job } => {
                let i = self.pick(job)?;
                let j = &mut self.jobs[i];
                if j.state != JobState::Submitted {
                    Outcome::Refused(Error::BadState)
                } else if !live(j) {
                    Outcome::Refused(Error::Expired)
                } else {
                    j.reason = Some(s.hash(9));
                    j.state = JobState::Completed;
                    self.escrow -= j.budget;
                    self.provider += j.budget;
                    Outcome::Ok
                }
            }
            Op::Reject { job } => {
                let i = self.pick(job)?;
                let j = &mut self.jobs[i];
                match j.state {
                    JobState::Open => {
                        j.reason = None;
                        j.state = JobState::Rejected;
                        Outcome::Ok
                    }
                    JobState::Funded | JobState::Submitted if !live(j) => {
                        Outcome::Refused(Error::Expired)
                    }
                    JobState::Funded | JobState::Submitted => {
                        j.reason = None;
                        j.state = JobState::Rejected;
                        self.escrow -= j.budget;
                        self.client += j.budget;
                        Outcome::Ok
                    }
                    _ => Outcome::Refused(Error::BadState),
                }
            }
            Op::ClaimRefund { job } => {
                let i = self.pick(job)?;
                let j = &mut self.jobs[i];
                if !j.state.holds_funds() {
                    Outcome::Refused(Error::BadState)
                } else if live(j) {
                    Outcome::Refused(Error::NotExpired)
                } else {
                    j.state = JobState::Expired;
                    self.escrow -= j.budget;
                    self.client += j.budget;
                    Outcome::Ok
                }
            }
            Op::Advance { .. } => return None,
        };
        Some(outcome)
    }
}

fn outcome<T, C>(result: Result<Result<T, C>, Result<Error, soroban_sdk::InvokeError>>) -> Outcome {
    match result {
        Ok(_) => Outcome::Ok,
        Err(Ok(error)) => Outcome::Refused(error),
        Err(Err(_)) => Outcome::Other,
    }
}

/// Runs `op` against the contract and reports what happened, or `None`
/// where the model would not act either (no job to pick, or a clock move).
fn run(s: &Setup, kernel: &CommerceKernelClient, op: &Op) -> Option<Outcome> {
    let count = kernel.job_count();
    let pick = |index: usize| (count > 0).then(|| index as u64 % count + 1);
    Some(match *op {
        Op::Create {
            with_provider,
            duration,
        } => outcome(kernel.try_create_job(
            &s.client,
            &with_provider.then(|| s.provider.clone()),
            &s.evaluator,
            &(s.now() + duration),
            &s.hash(1),
        )),
        Op::SetProvider { job } => outcome(kernel.try_set_provider(&pick(job)?, &s.provider)),
        Op::SetBudget {
            job,
            by_provider,
            amount,
        } => {
            let actor = if by_provider { &s.provider } else { &s.client };
            outcome(kernel.try_set_budget(&pick(job)?, actor, &amount))
        }
        Op::Fund { job, skew } => {
            let id = pick(job)?;
            let expected = kernel.get_job(&id).budget + skew;
            outcome(kernel.try_fund(&id, &expected))
        }
        Op::Submit { job } => outcome(kernel.try_submit(&pick(job)?, &s.hash(2))),
        Op::Complete { job } => outcome(kernel.try_complete(&pick(job)?, &s.hash(9))),
        Op::Reject { job } => outcome(kernel.try_reject(&pick(job)?, &None)),
        Op::ClaimRefund { job } => outcome(kernel.try_claim_refund(&pick(job)?)),
        Op::Advance { seconds } => {
            s.advance_time(seconds);
            return None;
        }
    })
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: std::env::var("PROPTEST_CASES").ok().and_then(|n| n.parse().ok()).unwrap_or(64),
        ..ProptestConfig::default()
    })]

    #[test]
    fn the_kernel_matches_a_plain_rust_model_of_the_spec(ops in prop::collection::vec(op(), 1..60)) {
        let s = Setup::without_snapshots();
        let kernel = s.kernel();
        let mut model = Model::new();

        for (step, op) in ops.iter().enumerate() {
            // The model reads the clock before the contract can move it.
            let predicted = model.step(&s, op);
            let actual = run(&s, &kernel, op);
            prop_assert_eq!(actual, predicted, "step {}: {:?}", step, op);

            let jobs: std::vec::Vec<Job> = all_jobs(&kernel).into_values().collect();
            prop_assert_eq!(&jobs, &model.jobs, "step {}: {:?}", step, op);
            prop_assert_eq!(s.balance(&s.client), model.client, "client after step {}", step);
            prop_assert_eq!(s.balance(&s.provider), model.provider, "provider after step {}", step);
            prop_assert_eq!(s.balance(&kernel.address), model.escrow, "escrow after step {}", step);
        }
    }
}
