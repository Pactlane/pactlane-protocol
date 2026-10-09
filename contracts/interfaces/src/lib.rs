//! The commerce kernel's public surface, shared by the kernel, the contracts
//! that call it, and tests.
//!
//! Everything here mirrors `specs/INTERFACES.md`. Error codes, enum
//! discriminants and event shapes are part of the deployed ABI: changing one
//! requires a new kernel version, not an edit.
#![no_std]

use soroban_sdk::{
    contractclient, contracterror, contractevent, contracttype, Address, BytesN, Env,
};

/// Longest a job may run, in seconds (90 days).
///
/// Half the network's maximum entry TTL (testnet `max_entry_ttl` is 3,110,400
/// ledgers, about 180 days), so a job's entry, extended on every write, cannot
/// archive before the job expires.
pub const MAX_JOB_DURATION: u64 = 90 * 24 * 60 * 60;

/// Where a job is in its lifecycle. `Completed`, `Rejected` and `Expired` are
/// terminal.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum JobState {
    Open = 0,
    Funded = 1,
    Submitted = 2,
    Completed = 3,
    Rejected = 4,
    Expired = 5,
}

impl JobState {
    /// Whether the kernel holds this job's budget.
    pub fn holds_funds(self) -> bool {
        matches!(self, Self::Funded | Self::Submitted)
    }

    /// Whether no further action can change the job.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Rejected | Self::Expired)
    }
}

/// One job's complete record.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Job {
    pub id: u64,
    pub client: Address,
    /// Set at most once, while the job is open.
    pub provider: Option<Address>,
    /// Fixed at creation.
    pub evaluator: Address,
    /// Commitment to the off-chain task specification.
    pub spec_hash: BytesN<32>,
    /// Token base units; zero until a budget is proposed.
    pub budget: i128,
    /// Ledger timestamp in seconds.
    pub expires_at: u64,
    pub state: JobState,
    /// Set by `submit`.
    pub work_hash: Option<BytesN<32>>,
    /// Set by `complete` or `reject`.
    pub reason: Option<BytesN<32>>,
}

/// Kernel errors. The codes are stable and never renumbered.
#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// No job with that ID.
    NotFound = 1,
    /// The action is not allowed in the job's current state.
    BadState = 2,
    /// `set_budget` caller is neither the client nor the provider.
    BadActor = 3,
    /// `expires_at` is not in the future, or exceeds [`MAX_JOB_DURATION`].
    BadExpiry = 4,
    /// The action needs a live job, but the job has reached `expires_at`.
    Expired = 5,
    /// `claim_refund` before `expires_at`.
    NotExpired = 6,
    /// The budget is not positive.
    BadBudget = 7,
    /// `expected_budget` differs from the stored budget.
    BudgetMismatch = 8,
    /// The job has no provider to fund.
    NoProvider = 9,
    /// The provider is already assigned.
    ProviderAlreadySet = 10,
    /// Two roles share one address.
    RoleConflict = 11,
    /// The job counter is exhausted.
    IdOverflow = 12,
}

/// The kernel's callable interface.
///
/// The kernel implements this trait, so its exported ABI cannot drift from
/// what callers compile against. Other contracts call the kernel through the
/// generated [`CommerceClient`] without linking the kernel's code.
#[contractclient(name = "CommerceClient")]
pub trait Commerce {
    fn create_job(
        env: Env,
        client: Address,
        provider: Option<Address>,
        evaluator: Address,
        expires_at: u64,
        spec_hash: BytesN<32>,
    ) -> Result<u64, Error>;
    fn set_provider(env: Env, id: u64, provider: Address) -> Result<(), Error>;
    fn set_budget(env: Env, id: u64, actor: Address, amount: i128) -> Result<(), Error>;
    fn fund(env: Env, id: u64, expected_budget: i128) -> Result<(), Error>;
    fn submit(env: Env, id: u64, work_hash: BytesN<32>) -> Result<(), Error>;
    fn complete(env: Env, id: u64, reason: BytesN<32>) -> Result<(), Error>;
    fn reject(env: Env, id: u64, reason: Option<BytesN<32>>) -> Result<(), Error>;
    fn claim_refund(env: Env, id: u64) -> Result<(), Error>;
    fn extend_ttl(env: Env, id: u64) -> Result<(), Error>;
    fn get_job(env: Env, id: u64) -> Result<Job, Error>;
    fn job_count(env: Env) -> u64;
    fn token(env: Env) -> Address;
}

// Events. Fixed topic names keep them stable across Rust renames. Events with
// an optional field use `sparse = false`, so indexers always see every key.

#[contractevent(topics = ["job_created"], sparse = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JobCreated {
    #[topic]
    pub id: u64,
    #[topic]
    pub client: Address,
    pub provider: Option<Address>,
    pub evaluator: Address,
    pub expires_at: u64,
    pub spec_hash: BytesN<32>,
}

#[contractevent(topics = ["provider_set"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderSet {
    #[topic]
    pub id: u64,
    #[topic]
    pub provider: Address,
}

#[contractevent(topics = ["budget_set"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BudgetSet {
    #[topic]
    pub id: u64,
    #[topic]
    pub actor: Address,
    pub amount: i128,
}

#[contractevent(topics = ["job_funded"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JobFunded {
    #[topic]
    pub id: u64,
    #[topic]
    pub client: Address,
    pub amount: i128,
}

#[contractevent(topics = ["job_submitted"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JobSubmitted {
    #[topic]
    pub id: u64,
    #[topic]
    pub provider: Address,
    pub work_hash: BytesN<32>,
}

#[contractevent(topics = ["job_completed"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JobCompleted {
    #[topic]
    pub id: u64,
    #[topic]
    pub evaluator: Address,
    pub reason: BytesN<32>,
}

#[contractevent(topics = ["job_rejected"], sparse = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JobRejected {
    #[topic]
    pub id: u64,
    #[topic]
    pub rejector: Address,
    pub reason: Option<BytesN<32>>,
}

#[contractevent(topics = ["job_expired"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JobExpired {
    #[topic]
    pub id: u64,
}

#[contractevent(topics = ["payment_released"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentReleased {
    #[topic]
    pub id: u64,
    #[topic]
    pub provider: Address,
    pub amount: i128,
}

#[contractevent(topics = ["refunded"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Refunded {
    #[topic]
    pub id: u64,
    #[topic]
    pub client: Address,
    pub amount: i128,
}
