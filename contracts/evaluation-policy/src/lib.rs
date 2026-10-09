//! An evaluation policy: a rotatable set of signers acting as one job
//! evaluator.
//!
//! The kernel fixes each job's evaluator at creation, so a single account key
//! could never be rotated on a live job. Jobs name this contract as their
//! evaluator instead, and the owner changes who may act for it.
//! `specs/EVALUATOR_TRUST.md` has the trust model and `specs/INTERFACES.md`
//! the interface.
#![no_std]

mod storage;

pub use storage::DataKey;

use pactlane_interfaces::CommerceClient;
use soroban_sdk::{contract, contracterror, contractevent, contractimpl, Address, BytesN, Env};

/// Policy errors. Codes start at 101 so they can never be mistaken for the
/// kernel's codes (1-12), which pass through settlement unchanged.
#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum PolicyError {
    /// The address is not in the signer set.
    NotSigner = 101,
    /// The address is already a signer.
    AlreadySigner = 102,
    /// The signer is the job's client or provider.
    SignerConflict = 103,
    /// No ownership transfer is awaiting acceptance.
    NoPendingOwner = 104,
}

#[contractevent(topics = ["signer_added"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignerAdded {
    #[topic]
    pub signer: Address,
}

#[contractevent(topics = ["signer_removed"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignerRemoved {
    #[topic]
    pub signer: Address,
}

#[contractevent(topics = ["owner_proposed"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnerProposed {
    #[topic]
    pub owner: Address,
    #[topic]
    pub pending: Address,
}

#[contractevent(topics = ["owner_changed"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnerChanged {
    #[topic]
    pub old_owner: Address,
    #[topic]
    pub new_owner: Address,
}

/// One settlement, attributed to the signer who made it. The kernel's own
/// events only show the policy as evaluator.
#[contractevent(topics = ["settled"], sparse = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Settled {
    #[topic]
    pub id: u64,
    #[topic]
    pub signer: Address,
    /// True for `complete`, false for `reject`.
    pub completed: bool,
    pub reason: Option<BytesN<32>>,
}

#[contract]
pub struct EvaluationPolicy;

#[contractimpl]
impl EvaluationPolicy {
    /// Sets the owner and binds the policy to one kernel forever.
    ///
    /// Calls the kernel's `job_count()` once so that binding to an address
    /// that is not a kernel fails at deploy time.
    pub fn __constructor(env: Env, owner: Address, kernel: Address) {
        CommerceClient::new(&env, &kernel).job_count();
        storage::set_owner(&env, &owner);
        storage::set_kernel(&env, &kernel);
    }

    /// Lets `signer` complete or reject jobs on this policy's behalf.
    pub fn add_signer(env: Env, signer: Address) -> Result<(), PolicyError> {
        storage::owner(&env).require_auth();
        if storage::is_signer(&env, &signer) {
            return Err(PolicyError::AlreadySigner);
        }
        storage::add_signer(&env, &signer);
        SignerAdded { signer }.publish(&env);
        Ok(())
    }

    /// Revokes `signer` for every job, including jobs funded before now.
    pub fn remove_signer(env: Env, signer: Address) -> Result<(), PolicyError> {
        storage::owner(&env).require_auth();
        if !storage::is_signer(&env, &signer) {
            return Err(PolicyError::NotSigner);
        }
        storage::remove_signer(&env, &signer);
        SignerRemoved { signer }.publish(&env);
        Ok(())
    }

    /// Starts an ownership transfer. Nothing changes until `new_owner`
    /// accepts, so a mistyped address cannot lose control of the policy.
    /// Proposing again replaces the pending proposal.
    pub fn propose_owner(env: Env, new_owner: Address) {
        let owner = storage::owner(&env);
        owner.require_auth();
        storage::set_pending_owner(&env, &new_owner);
        OwnerProposed {
            owner,
            pending: new_owner,
        }
        .publish(&env);
    }

    /// Completes an ownership transfer; only the proposed owner can call it.
    pub fn accept_owner(env: Env) -> Result<(), PolicyError> {
        let new_owner = storage::pending_owner(&env).ok_or(PolicyError::NoPendingOwner)?;
        new_owner.require_auth();

        let old_owner = storage::owner(&env);
        storage::set_owner(&env, &new_owner);
        storage::clear_pending_owner(&env);
        OwnerChanged {
            old_owner,
            new_owner,
        }
        .publish(&env);
        Ok(())
    }

    /// Approves job `id` and pays its provider, as this policy.
    ///
    /// The kernel applies all of its own rules; its errors pass through.
    pub fn complete(
        env: Env,
        signer: Address,
        id: u64,
        reason: BytesN<32>,
    ) -> Result<(), PolicyError> {
        let kernel = authorize_settlement(&env, &signer, id)?;
        kernel.complete(&id, &reason);
        Settled {
            id,
            signer,
            completed: true,
            reason: Some(reason),
        }
        .publish(&env);
        Ok(())
    }

    /// Rejects job `id` and refunds its client, as this policy.
    ///
    /// The kernel applies all of its own rules; its errors pass through.
    pub fn reject(
        env: Env,
        signer: Address,
        id: u64,
        reason: Option<BytesN<32>>,
    ) -> Result<(), PolicyError> {
        let kernel = authorize_settlement(&env, &signer, id)?;
        kernel.reject(&id, &reason);
        Settled {
            id,
            signer,
            completed: false,
            reason,
        }
        .publish(&env);
        Ok(())
    }

    /// Whether `address` may currently settle jobs for this policy.
    pub fn is_signer(env: Env, address: Address) -> bool {
        storage::is_signer(&env, &address)
    }

    /// Who manages the signer set.
    pub fn owner(env: Env) -> Address {
        storage::owner(&env)
    }

    /// The only kernel this policy settles jobs on.
    pub fn kernel(env: Env) -> Address {
        storage::kernel(&env)
    }
}

/// Checks that `signer` signed, is in the signer set, and is neither the job's
/// client nor its provider. Returns a client for the bound kernel.
///
/// The kernel only sees this policy as the evaluator, so it cannot tell when
/// the person behind the signature is a party to the job. Without this check
/// a provider who is also a signer could approve its own work (threat T1).
fn authorize_settlement<'a>(
    env: &'a Env,
    signer: &Address,
    id: u64,
) -> Result<CommerceClient<'a>, PolicyError> {
    signer.require_auth();
    if !storage::is_signer(env, signer) {
        return Err(PolicyError::NotSigner);
    }

    let kernel = CommerceClient::new(env, &storage::kernel(env));
    let job = kernel.get_job(&id);
    if &job.client == signer || job.provider.as_ref() == Some(signer) {
        return Err(PolicyError::SignerConflict);
    }

    storage::extend_signer(env, signer);
    Ok(kernel)
}
