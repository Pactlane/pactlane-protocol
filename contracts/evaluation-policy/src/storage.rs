//! Storage layout and TTL. Keys are append-only like the ABI. Every write
//! extends what it wrote to the network's maximum TTL.

use soroban_sdk::{contracttype, Address, Env};

/// Storage keys. Public so tests and auditors can inspect entries and TTLs.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Instance: who manages the signer set.
    Owner,
    /// Instance: an ownership transfer awaiting acceptance.
    PendingOwner,
    /// Instance: the one kernel this policy settles jobs on.
    Kernel,
    /// Persistent: present while the address is a signer.
    Signer(Address),
}

fn instance_get(env: &Env, key: &DataKey) -> Option<Address> {
    env.storage().instance().get(key)
}

fn instance_set(env: &Env, key: &DataKey, value: &Address) {
    env.storage().instance().set(key, value);
    extend_instance(env);
}

pub fn owner(env: &Env) -> Address {
    instance_get(env, &DataKey::Owner).expect("owner is set by the constructor")
}

pub fn set_owner(env: &Env, owner: &Address) {
    instance_set(env, &DataKey::Owner, owner);
}

pub fn pending_owner(env: &Env) -> Option<Address> {
    instance_get(env, &DataKey::PendingOwner)
}

pub fn set_pending_owner(env: &Env, pending: &Address) {
    instance_set(env, &DataKey::PendingOwner, pending);
}

pub fn clear_pending_owner(env: &Env) {
    env.storage().instance().remove(&DataKey::PendingOwner);
    extend_instance(env);
}

pub fn kernel(env: &Env) -> Address {
    instance_get(env, &DataKey::Kernel).expect("kernel is set by the constructor")
}

pub fn set_kernel(env: &Env, kernel: &Address) {
    instance_set(env, &DataKey::Kernel, kernel);
}

pub fn is_signer(env: &Env, address: &Address) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::Signer(address.clone()))
}

pub fn add_signer(env: &Env, signer: &Address) {
    let key = DataKey::Signer(signer.clone());
    env.storage().persistent().set(&key, &());
    extend_signer(env, signer);
}

pub fn remove_signer(env: &Env, signer: &Address) {
    env.storage()
        .persistent()
        .remove(&DataKey::Signer(signer.clone()));
    extend_instance(env);
}

/// Renews a signer's entry, and the instance, to the maximum TTL.
pub fn extend_signer(env: &Env, signer: &Address) {
    let max = env.storage().max_ttl();
    env.storage()
        .persistent()
        .extend_ttl(&DataKey::Signer(signer.clone()), max, max);
    extend_instance(env);
}

fn extend_instance(env: &Env) {
    let max = env.storage().max_ttl();
    env.storage().instance().extend_ttl(max, max);
}
