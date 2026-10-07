//! Block identity: heights, hashes and the pointer that pairs them.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use serde::{Deserialize, Serialize};

/// A block height. `u64` is deliberate: no chain in scope needs more, and a
/// fixed width keeps the mapping ABI encoding trivial.
pub type BlockNumber = u64;

/// A block hash, stored as raw bytes so every family can share the type.
///
/// EVM hashes are 32 bytes; Solana slots hash to 32 bytes; Stellar ledger
/// hashes are 32 bytes. The type stays variable-length anyway so an unusual
/// chain never needs a second hash type.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct BlockHash(pub Vec<u8>);

impl BlockHash {
    /// Wrap raw bytes.
    pub const fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// Parse a `0x`-prefixed or bare hex string.
    pub fn from_hex(s: &str) -> Result<Self, hex::FromHexError> {
        let trimmed = s.strip_prefix("0x").unwrap_or(s);
        hex::decode(trimmed).map(Self)
    }

    /// The `0x`-prefixed hex form used everywhere user-visible.
    pub fn to_hex(&self) -> String {
        let mut out = String::from("0x");
        out.push_str(&hex::encode(&self.0));
        out
    }

    /// Raw bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for BlockHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BlockHash({})", self.to_hex())
    }
}

impl fmt::Display for BlockHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl Serialize for BlockHash {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for BlockHash {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let s = String::deserialize(de)?;
        BlockHash::from_hex(&s).map_err(serde::de::Error::custom)
    }
}

/// A height paired with the hash observed at that height.
///
/// Both halves are required: the height alone cannot survive a reorg, which is
/// the whole reason the node stores pointers rather than numbers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockPtr {
    /// Height of the block.
    pub number: BlockNumber,
    /// Hash of the block at `number`.
    pub hash: BlockHash,
}

impl BlockPtr {
    /// Pair a height with its hash.
    pub const fn new(number: BlockNumber, hash: BlockHash) -> Self {
        Self { number, hash }
    }
}

impl fmt::Display for BlockPtr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{} ({})", self.number, self.hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trips_with_and_without_prefix() {
        let a = BlockHash::from_hex("0xdeadbeef").unwrap();
        let b = BlockHash::from_hex("deadbeef").unwrap();
        assert_eq!(a, b);
        assert_eq!(a.to_hex(), "0xdeadbeef");
    }

    #[test]
    fn block_ptr_serializes_hash_as_prefixed_hex() {
        let ptr = BlockPtr::new(21_000_000, BlockHash::from_hex("0x00ff").unwrap());
        let json = serde_json::to_string(&ptr).unwrap();
        assert_eq!(json, r#"{"number":21000000,"hash":"0x00ff"}"#);
        assert_eq!(serde_json::from_str::<BlockPtr>(&json).unwrap(), ptr);
    }
}
