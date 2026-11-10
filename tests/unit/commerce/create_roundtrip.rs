//! `create_job` stores its arguments exactly: across many varied spec
//! hashes, expiries and providers, `get_job` reads back what was given.

use pactlane_interfaces::{Job, JobState, MAX_JOB_DURATION};
use pactlane_tests::Setup;
use soroban_sdk::BytesN;

/// A small deterministic generator (SplitMix64), so failures reproduce.
struct Seq(u64);

impl Seq {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
}

#[test]
fn every_create_job_argument_reads_back_unchanged() {
    let s = Setup::without_snapshots();
    let kernel = s.kernel();
    let mut seq = Seq(8183);

    for round in 0..64u64 {
        let mut spec = [0u8; 32];
        for chunk in spec.chunks_mut(8) {
            chunk.copy_from_slice(&seq.next().to_be_bytes());
        }
        // Edge spec hashes alongside random ones.
        match round {
            0 => spec = [0; 32],
            1 => spec = [0xff; 32],
            _ => {}
        }
        let duration = match round {
            0 => 1,
            1 => MAX_JOB_DURATION,
            _ => 1 + seq.next() % MAX_JOB_DURATION,
        };
        let expires_at = s.now() + duration;
        let provider = (round % 3 != 0).then(|| s.provider.clone());
        let spec_hash = BytesN::from_array(&s.env, &spec);

        let id = kernel.create_job(&s.client, &provider, &s.evaluator, &expires_at, &spec_hash);

        assert_eq!(
            kernel.get_job(&id),
            Job {
                id,
                client: s.client.clone(),
                provider,
                evaluator: s.evaluator.clone(),
                spec_hash,
                budget: 0,
                expires_at,
                state: JobState::Open,
                work_hash: None,
                reason: None,
            },
            "round {round}"
        );
    }
}
