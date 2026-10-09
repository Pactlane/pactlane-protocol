//! Pactlane's ERC-8183 escrow kernel.
//!
//! `specs/INTERFACES.md` is the source of truth; this crate implements it.
//! There is deliberately no admin, upgrade, pause or hook: once deployed, the
//! rules below are the only way escrowed funds move.
#![no_std]

mod storage;

use pactlane_interfaces::{Error, Job, JobCreated, JobState, MAX_JOB_DURATION};
use soroban_sdk::{contract, contractimpl, token, Address, BytesN, Env};

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

    /// Opens a job. The evaluator is fixed here for the job's whole life.
    pub fn create_job(
        env: Env,
        client: Address,
        provider: Option<Address>,
        evaluator: Address,
        expires_at: u64,
        spec_hash: BytesN<32>,
    ) -> Result<u64, Error> {
        client.require_auth();

        let now = env.ledger().timestamp();
        // `expires_at > now` is checked first, so the subtraction cannot wrap.
        if expires_at <= now || expires_at - now > MAX_JOB_DURATION {
            return Err(Error::BadExpiry);
        }
        check_roles(&client, provider.as_ref(), &evaluator)?;

        let id = storage::job_count(&env)
            .checked_add(1)
            .ok_or(Error::IdOverflow)?;
        storage::set_job_count(&env, id);
        storage::put_job(
            &env,
            &Job {
                id,
                client: client.clone(),
                provider: provider.clone(),
                evaluator: evaluator.clone(),
                spec_hash: spec_hash.clone(),
                budget: 0,
                expires_at,
                state: JobState::Open,
                work_hash: None,
                reason: None,
            },
        );

        JobCreated {
            id,
            client,
            provider,
            evaluator,
            expires_at,
            spec_hash,
        }
        .publish(&env);
        Ok(id)
    }

    /// The job's full record.
    pub fn get_job(env: Env, id: u64) -> Result<Job, Error> {
        storage::job(&env, id)
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

/// Client, provider and evaluator must be three different addresses: no one
/// approves their own job or their own work, and no client pays itself.
fn check_roles(
    client: &Address,
    provider: Option<&Address>,
    evaluator: &Address,
) -> Result<(), Error> {
    if evaluator == client {
        return Err(Error::RoleConflict);
    }
    if let Some(provider) = provider {
        if provider == evaluator || provider == client {
            return Err(Error::RoleConflict);
        }
    }
    Ok(())
}
