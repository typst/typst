use std::hash::Hash;

use indexmap::IndexMap;

use crate::util::hash128;
use crate::vm::{
    AccessId, ClosureId, Constant, LabelId, PatternId, Pointer, ScopeId, SpanId, StringId,
};

pub struct Remapper<K, V> {
    values: IndexMap<u128, (K, V)>,
}

impl<K: RemapperKey, V: Hash> Default for Remapper<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: RemapperKey, V: Hash> Remapper<K, V> {
    /// Creates a new empty remapper.
    pub fn new() -> Self {
        Self { values: IndexMap::new() }
    }

    /// Inserts a new value into the remapper.
    pub fn insert(&mut self, value: V) -> K {
        let hash = hash128(&value);

        let len = self.values.len();
        let (key, _) = self.values.entry(hash).or_insert_with(|| {
            let key = K::from_raw(len as u16);
            (key, value)
        });

        key.clone()
    }

    pub fn into_values(&self) -> Vec<V>
    where
        V: Clone,
    {
        self.values.values().map(|(_, v)| v.clone()).collect()
    }
}

pub trait RemapperKey: Clone {
    fn as_raw(&self) -> u16;

    fn from_raw(raw: u16) -> Self;
}

impl RemapperKey for Constant {
    fn as_raw(&self) -> u16 {
        Self::as_raw(*self)
    }

    fn from_raw(raw: u16) -> Self {
        Self::new(raw)
    }
}

impl RemapperKey for StringId {
    fn as_raw(&self) -> u16 {
        Self::as_raw(*self)
    }

    fn from_raw(raw: u16) -> Self {
        Self::new(raw)
    }
}

impl RemapperKey for LabelId {
    fn as_raw(&self) -> u16 {
        Self::as_raw(*self)
    }

    fn from_raw(raw: u16) -> Self {
        Self::new(raw)
    }
}

impl RemapperKey for ClosureId {
    fn as_raw(&self) -> u16 {
        Self::as_raw(*self)
    }

    fn from_raw(raw: u16) -> Self {
        Self::new(raw)
    }
}

impl RemapperKey for AccessId {
    fn as_raw(&self) -> u16 {
        Self::as_raw(*self)
    }

    fn from_raw(raw: u16) -> Self {
        Self::new(raw)
    }
}

impl RemapperKey for PatternId {
    fn as_raw(&self) -> u16 {
        Self::as_raw(*self)
    }

    fn from_raw(raw: u16) -> Self {
        Self::new(raw)
    }
}

impl RemapperKey for ScopeId {
    fn as_raw(&self) -> u16 {
        Self::as_raw(*self)
    }

    fn from_raw(raw: u16) -> Self {
        Self::new(raw)
    }
}

impl RemapperKey for SpanId {
    fn as_raw(&self) -> u16 {
        Self::as_raw(*self)
    }

    fn from_raw(raw: u16) -> Self {
        Self::new(raw)
    }
}

impl RemapperKey for Pointer {
    fn as_raw(&self) -> u16 {
        Self::as_raw(*self)
    }

    fn from_raw(raw: u16) -> Self {
        Self::new(raw)
    }
}
