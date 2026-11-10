//! Policy settlements are attributable: every kernel settlement made
//! through the policy comes with exactly one `settled` event naming the
//! signer who made it, and a refused attempt publishes nothing. Over a
//! whole random sequence, the policy's `settled` count equals the number of
//! jobs the policy completed or rejected.

use pactlane_evaluation_policy::Settled;
use pactlane_interfaces::{JobCompleted, JobRejected, JobState};
use pactlane_tests::{Setup, BUDGET, JOB_DURATION};
use proptest::prelude::*;
use soroban_sdk::{testutils::Address as _, xdr::ContractEvent, Address, Event};

#[derive(Clone, Debug)]
enum Op {
    /// Opens and funds a job evaluated by the policy.
    Open,
    Submit {
        job: usize,
    },
    /// Settles through the policy as `who` (an index into the actor pool).
    Settle {
        job: usize,
        who: usize,
        complete: bool,
    },
    ClaimRefund {
        job: usize,
    },
    Advance {
        seconds: u64,
    },
}

fn op() -> impl Strategy<Value = Op> {
    let job = 0..6usize;
    prop_oneof![
        2 => Just(Op::Open),
        2 => job.clone().prop_map(|job| Op::Submit { job }),
        4 => (job.clone(), 0..4usize, any::<bool>())
            .prop_map(|(job, who, complete)| Op::Settle { job, who, complete }),
        1 => job.prop_map(|job| Op::ClaimRefund { job }),
        1 => (0..=JOB_DURATION).prop_map(|seconds| Op::Advance { seconds }),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: std::env::var("PROPTEST_CASES").ok().and_then(|n| n.parse().ok()).unwrap_or(64),
        ..ProptestConfig::default()
    })]

    #[test]
    fn every_policy_settlement_emits_exactly_one_attributed_settled_event(
        ops in prop::collection::vec(op(), 1..50)
    ) {
        let s = Setup::without_snapshots();
        s.token_admin.mint(&s.client, &(100 * BUDGET));
        let kernel = s.kernel();
        let policy = s.policy(&kernel.address, &Address::generate(&s.env));
        // Two real signers, an outsider, and a signer who is every job's client.
        let actors = [Address::generate(&s.env), Address::generate(&s.env), Address::generate(&s.env), s.client.clone()];
        for signer in [&actors[0], &actors[1], &actors[3]] {
            policy.add_signer(signer);
        }
        let mut jobs: std::vec::Vec<u64> = std::vec::Vec::new();
        let mut settled_total = 0usize;

        for op in &ops {
            let pick = |i: usize| (!jobs.is_empty()).then(|| jobs[i % jobs.len()]);
            match *op {
                Op::Open => jobs.push(s.funded_job_evaluated_by(&kernel, &policy.address)),
                Op::Submit { job } => if let Some(id) = pick(job) { let _ = kernel.try_submit(&id, &s.hash(2)); },
                Op::ClaimRefund { job } => if let Some(id) = pick(job) { let _ = kernel.try_claim_refund(&id); },
                Op::Advance { seconds } => s.advance_time(seconds),
                Op::Settle { job, who, complete } => {
                    let Some(id) = pick(job) else { continue };
                    let signer = &actors[who];
                    let reason = complete.then(|| s.hash(9));
                    let ok = if complete {
                        policy.try_complete(signer, &id, &s.hash(9)).is_ok()
                    } else {
                        policy.try_reject(signer, &id, &None).is_ok()
                    };
                    let from_policy = s.events_of(&policy.address);
                    let from_kernel: std::vec::Vec<ContractEvent> = s.events_of(&kernel.address).events().to_vec();
                    if ok {
                        settled_total += 1;
                        prop_assert_eq!(
                            from_policy,
                            [Settled { id, signer: signer.clone(), completed: complete, reason }
                                .to_xdr(&s.env, &policy.address)],
                            "{:?}", op
                        );
                        let decision = if complete {
                            JobCompleted { id, evaluator: policy.address.clone(), reason: s.hash(9) }.to_xdr(&s.env, &kernel.address)
                        } else {
                            JobRejected { id, rejector: policy.address.clone(), reason: None }.to_xdr(&s.env, &kernel.address)
                        };
                        prop_assert_eq!(from_kernel.iter().filter(|e| **e == decision).count(), 1, "{:?}", op);
                    } else {
                        prop_assert!(from_policy.events().is_empty(), "refused {:?} emitted settled", op);
                        prop_assert!(from_kernel.is_empty(), "refused {:?} moved the kernel", op);
                    }
                }
            }
        }

        let settled_by_policy = jobs
            .iter()
            .filter(|id| {
                let job = kernel.get_job(id);
                job.state == JobState::Completed || (job.state == JobState::Rejected)
            })
            .count();
        prop_assert_eq!(settled_total, settled_by_policy);
    }
}
