//! Entity identity and the untyped entity representation.
//!
//! Generated code produces typed structs; this is what those structs lower to
//! when they cross the mapping ABI. The node never sees a project's Rust
//! types, only [`Entity`] keyed by [`EntityKey`].

use alloc::string::{String, ToString};
use core::fmt;

use alloc::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::scalar::Value;

// Sorted, not insertion-ordered: entity field order must be canonical so the
// same logical entity hashes identically no matter what order a mapping set
// its fields in.
type Fields = BTreeMap<String, Value>;

/// An entity's primary key — the `id: ID!` field, always a string.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EntityId(String);

impl EntityId {
    /// Wrap an id. Empty ids are rejected because they collide in the store.
    pub fn new(id: impl Into<String>) -> Result<Self, InvalidEntityId> {
        let id = id.into();
        if id.is_empty() {
            return Err(InvalidEntityId);
        }
        Ok(Self(id))
    }

    /// The id as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// An entity id was empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidEntityId;

impl fmt::Display for InvalidEntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("entity id must not be empty")
    }
}

impl std::error::Error for InvalidEntityId {}

/// The store address of a single entity: its type plus its id.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityKey {
    /// The entity type name, exactly as written in `schema.graphql`.
    pub entity: String,
    /// The entity's `id`.
    pub id: EntityId,
}

impl EntityKey {
    /// Address an entity by type name and id.
    pub fn new(entity: impl Into<String>, id: EntityId) -> Self {
        Self {
            entity: entity.into(),
            id,
        }
    }
}

impl fmt::Display for EntityKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}[{}]", self.entity, self.id)
    }
}

/// An untyped entity: field name to value, as stored and as transported.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Entity(Fields);

impl Entity {
    /// An entity with no fields set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a field, returning the previous value if there was one.
    pub fn set(&mut self, field: impl Into<String>, value: impl Into<Value>) -> Option<Value> {
        self.0.insert(field.into(), value.into())
    }

    /// Read a field.
    pub fn get(&self, field: &str) -> Option<&Value> {
        self.0.get(field)
    }

    /// The `id` field, if it is present and a string.
    pub fn id(&self) -> Option<EntityId> {
        self.get("id")
            .and_then(Value::as_str)
            .map(ToString::to_string)
            .and_then(|s| EntityId::new(s).ok())
    }

    /// Iterate fields in canonical (sorted) order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Number of fields set.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether no fields are set.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl FromIterator<(String, Value)> for Entity {
    fn from_iter<T: IntoIterator<Item = (String, Value)>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_an_empty_id() {
        assert!(EntityId::new("").is_err());
        assert!(EntityId::new("0xabc-1").is_ok());
    }

    #[test]
    fn entity_reads_back_its_own_id() {
        let mut e = Entity::new();
        e.set("id", "0xabc-1");
        e.set("value", Value::BigInt("42".into()));
        assert_eq!(e.id().unwrap().as_str(), "0xabc-1");
        assert_eq!(e.len(), 2);
    }

    #[test]
    fn fields_iterate_in_canonical_order() {
        let mut e = Entity::new();
        e.set("value", 1i32);
        e.set("id", "x");
        e.set("from", "y");
        let names: Vec<_> = e.iter().map(|(k, _)| k).collect();
        assert_eq!(names, ["from", "id", "value"], "canonical order is sorted");
    }

    #[test]
    fn entity_key_displays_readably() {
        let key = EntityKey::new("Transfer", EntityId::new("0x1-0").unwrap());
        assert_eq!(key.to_string(), "Transfer[0x1-0]");
    }
}
