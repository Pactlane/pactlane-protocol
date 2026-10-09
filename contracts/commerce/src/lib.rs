//! Pactlane's ERC-8183 escrow kernel.
//!
//! `specs/INTERFACES.md` is the source of truth; this crate implements it.
//! There is deliberately no admin, upgrade, pause or hook: once deployed, the
//! rules below are the only way escrowed funds move.
#![no_std]

mod storage;

use pactlane_interfaces::{
    BudgetSet, Error, Job, JobCompleted, JobCreated, JobFunded, JobRejected, JobState,
    JobSubmitted, PaymentReleased, ProviderSet, Refunded, MAX_JOB_DURATION,
};
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

    /// Assigns the provider of an open job that has none yet.
    pub fn set_provider(env: Env, id: u64, provider: Address) -> Result<(), Error> {
        let mut job = storage::job(&env, id)?;
        job.client.require_auth();

        require_state(&job, JobState::Open)?;
        require_live(&env, &job)?;
        if job.provider.is_some() {
            return Err(Error::ProviderAlreadySet);
        }
        check_roles(&job.client, Some(&provider), &job.evaluator)?;

        job.provider = Some(provider.clone());
        storage::put_job(&env, &job);
        ProviderSet { id, provider }.publish(&env);
        Ok(())
    }

    /// Proposes the job's budget. Either party may propose, and a later
    /// proposal replaces an earlier one; `fund` is where the client agrees.
    pub fn set_budget(env: Env, id: u64, actor: Address, amount: i128) -> Result<(), Error> {
        let mut job = storage::job(&env, id)?;
        actor.require_auth();

        if actor != job.client && Some(&actor) != job.provider.as_ref() {
            return Err(Error::BadActor);
        }
        require_state(&job, JobState::Open)?;
        require_live(&env, &job)?;
        if amount <= 0 {
            return Err(Error::BadBudget);
        }

        job.budget = amount;
        storage::put_job(&env, &job);
        BudgetSet { id, actor, amount }.publish(&env);
        Ok(())
    }

    /// Moves the agreed budget from the client into escrow.
    ///
    /// `expected_budget` is the amount the client agreed to. If the stored
    /// budget changed before this call landed, funding fails rather than
    /// taking a different amount.
    pub fn fund(env: Env, id: u64, expected_budget: i128) -> Result<(), Error> {
        let mut job = storage::job(&env, id)?;
        job.client.require_auth();

        require_state(&job, JobState::Open)?;
        require_live(&env, &job)?;
        if job.provider.is_none() {
            return Err(Error::NoProvider);
        }
        if job.budget <= 0 {
            return Err(Error::BadBudget);
        }
        if job.budget != expected_budget {
            return Err(Error::BudgetMismatch);
        }

        // State first, transfer second.
        job.state = JobState::Funded;
        storage::put_job(&env, &job);
        token::TokenClient::new(&env, &storage::token(&env)).transfer(
            &job.client,
            env.current_contract_address(),
            &job.budget,
        );

        JobFunded {
            id,
            client: job.client,
            amount: job.budget,
        }
        .publish(&env);
        Ok(())
    }

    /// Records the provider's deliverable commitment on a funded job.
    pub fn submit(env: Env, id: u64, work_hash: BytesN<32>) -> Result<(), Error> {
        let mut job = storage::job(&env, id)?;
        // Only an open job can lack a provider, and open jobs cannot be
        // submitted, so this is the state error, not an authorization one.
        let provider = job.provider.clone().ok_or(Error::BadState)?;
        provider.require_auth();

        require_state(&job, JobState::Funded)?;
        require_live(&env, &job)?;

        job.work_hash = Some(work_hash.clone());
        job.state = JobState::Submitted;
        storage::put_job(&env, &job);
        JobSubmitted {
            id,
            provider,
            work_hash,
        }
        .publish(&env);
        Ok(())
    }

    /// Approves submitted work and pays the provider the full budget.
    ///
    /// `reason` is the hash of the evaluation evidence; every payout is bound
    /// to one.
    pub fn complete(env: Env, id: u64, reason: BytesN<32>) -> Result<(), Error> {
        let mut job = storage::job(&env, id)?;
        job.evaluator.require_auth();

        require_state(&job, JobState::Submitted)?;
        require_live(&env, &job)?;
        // Submitted implies funded, and funding requires a provider.
        let provider = job.provider.clone().ok_or(Error::NoProvider)?;

        // State first, transfer second.
        job.reason = Some(reason.clone());
        job.state = JobState::Completed;
        storage::put_job(&env, &job);
        token::TokenClient::new(&env, &storage::token(&env)).transfer(
            &env.current_contract_address(),
            &provider,
            &job.budget,
        );

        JobCompleted {
            id,
            evaluator: job.evaluator,
            reason,
        }
        .publish(&env);
        PaymentReleased {
            id,
            provider,
            amount: job.budget,
        }
        .publish(&env);
        Ok(())
    }

    /// Ends a job without payment.
    ///
    /// While the job is open it holds no funds, and its client may cancel it
    /// at any time. Once funded, only the evaluator may reject it, and only
    /// while it is live; the client gets the full budget back.
    pub fn reject(env: Env, id: u64, reason: Option<BytesN<32>>) -> Result<(), Error> {
        let mut job = storage::job(&env, id)?;
        let rejector = match job.state {
            JobState::Open => job.client.clone(),
            JobState::Funded | JobState::Submitted => job.evaluator.clone(),
            JobState::Completed | JobState::Rejected | JobState::Expired => {
                return Err(Error::BadState)
            }
        };
        rejector.require_auth();

        let refund = job.state.holds_funds();
        if refund {
            require_live(&env, &job)?;
        }

        // State first, transfer second.
        job.reason = reason.clone();
        job.state = JobState::Rejected;
        storage::put_job(&env, &job);
        if refund {
            token::TokenClient::new(&env, &storage::token(&env)).transfer(
                &env.current_contract_address(),
                &job.client,
                &job.budget,
            );
        }

        JobRejected {
            id,
            rejector,
            reason,
        }
        .publish(&env);
        if refund {
            Refunded {
                id,
                client: job.client,
                amount: job.budget,
            }
            .publish(&env);
        }
        Ok(())
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

fn require_state(job: &Job, expected: JobState) -> Result<(), Error> {
    if job.state == expected {
        Ok(())
    } else {
        Err(Error::BadState)
    }
}

/// A job is live strictly before `expires_at`. From that instant on, only a
/// refund can act on it, so payout and refund are never both possible.
fn require_live(env: &Env, job: &Job) -> Result<(), Error> {
    if env.ledger().timestamp() < job.expires_at {
        Ok(())
    } else {
        Err(Error::Expired)
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
