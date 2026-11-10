//! The policy's signer set behaves like a plain set. Random add and remove
//! sequences over a small pool of addresses are mirrored in a `BTreeSet`;
//! after every step each call's outcome and every address's `is_signer`
//! must match the model.

use std::collections::BTreeSet;

use pactlane_evaluation_policy::PolicyError;
use pactlane_tests::Setup;
use proptest::prelude::*;
use soroban_sdk::{testutils::Address as _, Address};

const POOL: usize = 5;

#[derive(Clone, Debug)]
enum Op {
    Add(usize),
    Remove(usize),
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![(0..POOL).prop_map(Op::Add), (0..POOL).prop_map(Op::Remove),]
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: std::env::var("PROPTEST_CASES").ok().and_then(|n| n.parse().ok()).unwrap_or(64),
        ..ProptestConfig::default()
    })]

    #[test]
    fn is_signer_always_matches_a_model_set(ops in prop::collection::vec(op(), 1..80)) {
        let s = Setup::without_snapshots();
        let kernel = s.kernel();
        let policy = s.policy(&kernel.address, &Address::generate(&s.env));
        let pool: std::vec::Vec<Address> = (0..POOL).map(|_| Address::generate(&s.env)).collect();
        let mut model = BTreeSet::new();

        for op in &ops {
            match *op {
                Op::Add(i) => {
                    let expected = if model.insert(i) { Ok(()) } else { Err(PolicyError::AlreadySigner) };
                    let actual = policy.try_add_signer(&pool[i]).map(|_| ()).map_err(|e| e.unwrap());
                    prop_assert_eq!(actual, expected, "{:?}", op);
                }
                Op::Remove(i) => {
                    let expected = if model.remove(&i) { Ok(()) } else { Err(PolicyError::NotSigner) };
                    let actual = policy.try_remove_signer(&pool[i]).map(|_| ()).map_err(|e| e.unwrap());
                    prop_assert_eq!(actual, expected, "{:?}", op);
                }
            }
            for (i, address) in pool.iter().enumerate() {
                prop_assert_eq!(policy.is_signer(address), model.contains(&i), "address {} after {:?}", i, op);
            }
        }
    }
}
