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
use soroban_sdk::{contract, contracterror, contractevent, contractimpl, Address, Env};

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
