use std::collections::HashMap;
use std::hash::Hash;

use indexmap::IndexMap;
use typst_syntax::Span;

use crate::foundations::{Label, Value};
use crate::lang::compiled::CompiledClosure;
use crate::lang::operands::{
    AccessId, ClosureId, Constant, LabelId, ModuleId, PatternId, Pointer, SpanId,
    StringId,
};
use crate::utils::hash128;

pub struct Remapper<K, V> {
    values: IndexMap<u128, (K, V)>,
    map: HashMap<K, u128>,
}

impl<K: RemapperKey, V: Hash> Default for Remapper<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: RemapperKey, V: Hash> Remapper<K, V> {
    /// Creates a new empty remapper.
    pub fn new() -> Self {
        Self { values: IndexMap::new(), map: HashMap::new() }
    }

    /// Inserts a new value into the remapper.
    pub fn insert(&mut self, value: V) -> K {
        let hash = hash128(&value);

        let len = self.values.len();
        let (key, _) = self.values.entry(hash).or_insert_with(|| {
            let key = K::from_raw(len as u16);
            (key, value)
        });

        self.map.insert(key.clone(), hash);

        key.clone()
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        let hash = self.map.get(key)?;
        self.values.get(hash).map(|(_, v)| v)
    }

    pub fn get_index(&self, index: usize) -> &V {
        self.values
            .get_index(index)
            .map(|(_, (_, v))| v)
            .expect("index out of bounds")
    }

    pub fn into_values(&self) -> Vec<V::CompiledValue>
    where
        V: Clone + IntoCompiledValue,
    {
        let mut out = Vec::from_iter(
            self.values.values().map(|(_, v)| v.clone().into_compiled_value()),
        );

        // A vec will grow to the next power of two, so we shrink it to the actual size.
        out.shrink_to_fit();

        out
    }
}

pub trait RemapperKey: Clone + Hash + Eq {
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

impl RemapperKey for ModuleId {
    fn as_raw(&self) -> u16 {
        Self::as_raw(*self)
    }

    fn from_raw(raw: u16) -> Self {
        Self::new(raw)
    }
}

pub trait IntoCompiledValue {
    type CompiledValue;

    fn into_compiled_value(self) -> Self::CompiledValue;
}

impl IntoCompiledValue for Value {
    type CompiledValue = Self;

    fn into_compiled_value(self) -> Self::CompiledValue {
        self
    }
}

impl IntoCompiledValue for Label {
    type CompiledValue = Label;

    fn into_compiled_value(self) -> Self::CompiledValue {
        self
    }
}

impl IntoCompiledValue for Span {
    type CompiledValue = Span;

    fn into_compiled_value(self) -> Self::CompiledValue {
        self
    }
}

impl IntoCompiledValue for CompiledClosure {
    type CompiledValue = CompiledClosure;

    fn into_compiled_value(self) -> Self::CompiledValue {
        self
    }
}
