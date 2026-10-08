//! Placeholder contract.
//!
//! It exists so the build, test, deploy and bindings scripts have something real
//! to run against before the commerce and evaluation-policy contracts land.
//! Delete it once a real contract covers the same pipeline.
#![no_std]

use soroban_sdk::{contract, contractimpl, vec, Env, String, Vec};

#[contract]
pub struct HelloWorld;

#[contractimpl]
impl HelloWorld {
    /// Returns `["Hello", to]`.
    pub fn hello(env: Env, to: String) -> Vec<String> {
        vec![&env, String::from_str(&env, "Hello"), to]
    }
}
