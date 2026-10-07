//! Core types shared across every SuperQuery component.
//!
//! This crate is the bottom of the dependency graph. It must stay small,
//! dependency-light and wasm-friendly: the guest mapping SDK, the CLI, the
//! node runtime and the query service all link against it, so a breaking
//! change here is a breaking change everywhere.
//!
//! Milestone 1 of `docs/IMPLEMENTATION_PLAN.md`.

#![forbid(unsafe_code)]

extern crate alloc;

pub mod abi_version;
pub mod block;
pub mod chain;
pub mod entity;
pub mod id;
pub mod newtypes;
pub mod scalar;
pub mod spec_version;

pub use abi_version::{MAPPING_ABI_VERSION, MappingAbiVersion};
pub use block::{BlockHash, BlockNumber, BlockPtr};
pub use chain::ChainFamily;
pub use entity::{Entity, EntityId, EntityKey};
pub use id::ProjectId;
pub use newtypes::{BigDecimal, BigInt, Bytes, Json, Timestamp};
pub use scalar::{ScalarKind, Value};
pub use spec_version::SpecVersion;

/// Everything a downstream crate normally needs in one import.
pub mod prelude {
    pub use crate::{
        BigDecimal, BigInt, BlockHash, BlockNumber, BlockPtr, Bytes, ChainFamily, Entity, EntityId,
        EntityKey, Json, MAPPING_ABI_VERSION, MappingAbiVersion, ProjectId, ScalarKind,
        SpecVersion, Timestamp, Value,
    };
}
