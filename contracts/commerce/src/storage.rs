//! Storage layout and TTL. Changing a key's shape strands existing data, so
//! keys are append-only like the ABI.
//!
//! Every write extends what it wrote to the network's maximum TTL. Jobs last
//! at most half that long, so a live job cannot archive between writes.

use pactlane_interfaces::{Error, Job};
use soroban_sdk::{contracttype, Address, Env};

/// Storage keys. Public so tests and auditors can inspect entries and TTLs.
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
    extend_instance(env);
}

pub fn job_count(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::JobCount)
        .unwrap_or(0)
}

pub fn set_job_count(env: &Env, count: u64) {
    env.storage().instance().set(&DataKey::JobCount, &count);
    extend_instance(env);
}

pub fn job(env: &Env, id: u64) -> Result<Job, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::Job(id))
        .ok_or(Error::NotFound)
}

pub fn put_job(env: &Env, job: &Job) {
    env.storage().persistent().set(&DataKey::Job(job.id), job);
    extend_job(env, job.id);
}

/// Extends a job's entry, and the instance it depends on, to the maximum TTL.
pub fn extend_job(env: &Env, id: u64) {
    let max = env.storage().max_ttl();
    env.storage()
        .persistent()
        .extend_ttl(&DataKey::Job(id), max, max);
    extend_instance(env);
}

/// Extends the contract instance (and its code) to the maximum TTL. Without
/// the instance, no job can be read or settled.
fn extend_instance(env: &Env) {
    let max = env.storage().max_ttl();
    env.storage().instance().extend_ttl(max, max);
}
