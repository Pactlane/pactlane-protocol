//! Pactlane's ERC-8183 escrow kernel.
//!
//! `specs/INTERFACES.md` is the source of truth; this crate implements it.
//! There is deliberately no admin, upgrade, pause or hook: once deployed, the
//! rules below are the only way escrowed funds move.
#![no_std]

mod storage;

use soroban_sdk::{contract, contractimpl, token, Address, Env};

#[contract]
pub struct CommerceKernel;

#[contractimpl]
impl CommerceKernel {
    /// Binds the payment token for the life of this deployment.
    ///
    /// Calls `decimals()` once so that deploying against an address that is
    /// not a token contract fails at deploy time, not at the first `fund`.
    pub fn __constructor(env: Env, token: Address) {
        token::TokenClient::new(&env, &token).decimals();
        storage::set_token(&env, &token);
    }

    /// The payment token every job in this deployment uses.
    pub fn token(env: Env) -> Address {
        storage::token(&env)
    }

    /// Number of jobs ever created, which is also the highest job ID.
    pub fn job_count(env: Env) -> u64 {
        storage::job_count(&env)
    }
}
