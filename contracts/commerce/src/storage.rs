//! Storage layout. Changing a key's shape strands existing data, so keys are
//! append-only like the ABI.

use soroban_sdk::{contracttype, Address, Env};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Instance: the payment token, set once by the constructor.
    Token,
    /// Instance: number of jobs ever created, which is also the last job ID.
    JobCount,
}

pub fn token(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Token)
        .expect("token is set by the constructor")
}

pub fn set_token(env: &Env, token: &Address) {
    env.storage().instance().set(&DataKey::Token, token);
}

pub fn job_count(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::JobCount)
        .unwrap_or(0)
}
