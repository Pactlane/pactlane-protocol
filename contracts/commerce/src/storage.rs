//! Storage layout. Changing a key's shape strands existing data, so keys are
//! append-only like the ABI.

use pactlane_interfaces::{Error, Job};
use soroban_sdk::{contracttype, Address, Env};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Instance: the payment token, set once by the constructor.
    Token,
    /// Instance: number of jobs ever created, which is also the last job ID.
    JobCount,
    /// Persistent: one entry per job.
    Job(u64),
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

pub fn set_job_count(env: &Env, count: u64) {
    env.storage().instance().set(&DataKey::JobCount, &count);
}

pub fn job(env: &Env, id: u64) -> Result<Job, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::Job(id))
        .ok_or(Error::NotFound)
}

pub fn put_job(env: &Env, job: &Job) {
    env.storage().persistent().set(&DataKey::Job(job.id), job);
}
