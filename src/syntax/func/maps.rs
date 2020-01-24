//! Deduplicating maps and keys for argument parsing.

use std::collections::HashMap;
use std::hash::Hash;
use crate::layout::{LayoutAxes, SpecificAxis, GenericAxis};
use crate::size::{PSize, ValueBox};
use super::*;


/// A deduplicating map type useful for storing possibly redundant arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct DedupMap<K, V> where K: Eq {
    map: Vec<Spanned<(K, V)>>,
}

impl<K, V> DedupMap<K, V> where K: Eq {
    pub fn new() -> DedupMap<K, V> {
        DedupMap { map: vec![] }
    }

    pub fn from_iter<I>(errors: &mut Errors, iter: I) -> DedupMap<K, V>
    where I: IntoIterator<Item=Spanned<(K, V)>> {
        let mut map = DedupMap::new();
        for Spanned { v: (key, value), span } in iter.into_iter() {
            map.insert(errors, key, value, span);
        }
        map
    }

    /// Add a key-value pair.
    pub fn insert(&mut self, errors: &mut Errors, key: K, value: V, span: Span) {
        if self.map.iter().any(|e| e.v.0 == key) {
            errors.push(err!(span; "duplicate argument"));
        } else {
            self.map.push(Spanned { v: (key, value), span });
        }
    }

    /// Add multiple key-value pairs.
    pub fn extend<I>(&mut self, errors: &mut Errors, items: I)
    where I: IntoIterator<Item=Spanned<(K, V)>> {
        for Spanned { v: (k, v), span } in items.into_iter() {
            self.insert(errors, k, v, span);
        }
    }

    /// Get the value corresponding to a key if it is present.
    pub fn get(&self, key: K) -> Option<&V> {
        self.map.iter().find(|e| e.v.0 == key).map(|e| &e.v.1)
    }

    /// Get the value and its span corresponding to a key if it is present.
    pub fn get_spanned(&self, key: K) -> Option<Spanned<&V>> {
        self.map.iter().find(|e| e.v.0 == key)
            .map(|e| Spanned { v: &e.v.1, span: e.span })
    }

    /// Call a function with the value if the key is present.
    pub fn with<F>(&self, key: K, callback: F) where F: FnOnce(&V) {
        if let Some(value) = self.get(key) {
            callback(value);
        }
    }

    /// Create a new map where keys and values are mapped to new keys and
    /// values.
    ///
    /// Returns an error if a new key is duplicate.
    pub fn dedup<F, K2, V2>(&self, errors: &mut Errors, mut f: F) -> DedupMap<K2, V2>
    where F: FnMut(&K, &V) -> (K2, V2), K2: Eq {
        let mut map = DedupMap::new();

        for Spanned { v: (key, value), span } in self.map.iter() {
            let (key, value) = f(key, value);
            map.insert(errors, key, value, *span);
        }

        map
    }

    /// Iterate over the (key, value) pairs.
    pub fn iter(&self) -> impl Iterator<Item=&(K, V)> {
        self.map.iter().map(|e| &e.v)
    }
}

/// A map for storing a value for two axes given by keyword arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisMap<V>(DedupMap<AxisKey, V>);

impl<V: Clone> AxisMap<V> {
    pub fn parse<KT: Key<Output=AxisKey>, VT: Value<Output=V>>(
        errors: &mut Errors,
        object: &mut Object,
    ) -> AxisMap<V> {
        let values: Vec<_> = object.get_all_spanned::<KT, VT>(errors).collect();
        AxisMap(DedupMap::from_iter(errors, values))
    }

    /// Deduplicate from specific or generic to just specific axes.
    pub fn dedup(&self, errors: &mut Errors, axes: LayoutAxes) -> DedupMap<SpecificAxis, V> {
        self.0.dedup(errors, |key, val| (key.to_specific(axes), val.clone()))
    }
}

/// A map for extracting values for two axes that are given through two
/// positional or keyword arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct PosAxisMap<V>(DedupMap<PosAxisKey, V>);

impl<V: Clone> PosAxisMap<V> {
    pub fn parse<KT: Key<Output=AxisKey>, VT: Value<Output=V>>(
        errors: &mut Errors,
        args: &mut FuncArgs,
    ) -> PosAxisMap<V> {
        let mut map = DedupMap::new();

        for &key in &[PosAxisKey::First, PosAxisKey::Second] {
            if let Some(value) = args.pos.get::<Spanned<VT>>(errors) {
                map.insert(errors, key, value.v, value.span);
            }
        }

        let keywords: Vec<_> = args.key
            .get_all_spanned::<KT, VT>(errors)
            .map(|s| s.map(|(k, v)| (PosAxisKey::Keyword(k), v)))
            .collect();

        map.extend(errors, keywords);

        PosAxisMap(map)
    }

    /// Deduplicate from positional or specific to generic axes.
    pub fn dedup<F>(
        &self,
        errors: &mut Errors,
        axes: LayoutAxes,
        mut f: F,
    ) -> DedupMap<GenericAxis, V> where F: FnMut(&V) -> Option<GenericAxis> {
        self.0.dedup(errors, |key, val| {
            (match key {
                PosAxisKey::First => f(val).unwrap_or(GenericAxis::Primary),
                PosAxisKey::Second => f(val).unwrap_or(GenericAxis::Secondary),
                PosAxisKey::Keyword(AxisKey::Specific(axis)) => axis.to_generic(axes),
                PosAxisKey::Keyword(AxisKey::Generic(axis)) => *axis,
            }, val.clone())
        })
    }
}

/// A map for extracting padding for a set of specifications given for all
/// sides, opposing sides or single sides.
#[derive(Debug, Clone, PartialEq)]
pub struct PaddingMap(DedupMap<PaddingKey<AxisKey>, Option<PSize>>);

impl PaddingMap {
    pub fn parse(errors: &mut Errors, args: &mut FuncArgs) -> PaddingMap {
        let mut map = DedupMap::new();

        if let Some(psize) = args.pos.get::<Spanned<Defaultable<PSize>>>(errors) {
            map.insert(errors, PaddingKey::All, psize.v, psize.span);
        }

        let paddings: Vec<_> = args.key
            .get_all_spanned::<PaddingKey<AxisKey>, Defaultable<PSize>>(errors)
            .collect();

        map.extend(errors, paddings);

        PaddingMap(map)
    }

    /// Apply the specified padding on a value box of optional, scalable sizes.
    pub fn apply(
        &self,
        errors: &mut Errors,
        axes: LayoutAxes,
        padding: &mut ValueBox<Option<PSize>>
    ) {
        use PaddingKey::*;
        use SpecificAxis::*;

        let map = self.0.dedup(errors, |key, &val| {
            (match key {
                All => All,
                Both(axis) => Both(axis.to_specific(axes)),
                Side(axis, alignment) => {
                    let axis = axis.to_specific(axes);
                    Side(axis, alignment.to_specific(axes, axis))
                }
            }, val)
        });

        map.with(All, |&val| padding.set_all(val));
        map.with(Both(Horizontal), |&val| padding.set_horizontal(val));
        map.with(Both(Vertical), |&val| padding.set_vertical(val));

        for &(key, val) in map.iter() {
            if let Side(_, alignment) = key {
                match alignment {
                    AlignmentValue::Left => padding.left = val,
                    AlignmentValue::Right => padding.right = val,
                    AlignmentValue::Top => padding.top = val,
                    AlignmentValue::Bottom => padding.bottom = val,
                    _ => {},
                }
            }
        }
    }
}
