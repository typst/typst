//! Deduplicating maps and keys for argument parsing.

use crate::problem::Problems;
use crate::layout::prelude::*;
use crate::size::{PSize, ValueBox};
use crate::syntax::span::Spanned;
use super::keys::*;
use super::values::*;
use super::*;


/// A map which deduplicates redundant arguments.
///
/// Whenever a duplicate argument is inserted into the map, through the
/// functions `from_iter`, `insert` or `extend` an problems is added to the error
/// list that needs to be passed to those functions.
///
/// All entries need to have span information to enable the error reporting.
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct DedupMap<K, V> where K: Eq {
    map: Vec<Spanned<(K, V)>>,
}

impl<K, V> DedupMap<K, V> where K: Eq {
    /// Create a new deduplicating map.
    pub fn new() -> DedupMap<K, V> {
        DedupMap { map: vec![] }
    }

    /// Create a new map from an iterator of spanned keys and values.
    pub fn from_iter<I>(problems: &mut Problems, iter: I) -> DedupMap<K, V>
    where I: IntoIterator<Item=Spanned<(K, V)>> {
        let mut map = DedupMap::new();
        map.extend(problems, iter);
        map
    }

    /// Add a spanned key-value pair.
    pub fn insert(&mut self, problems: &mut Problems, entry: Spanned<(K, V)>) {
        if self.map.iter().any(|e| e.v.0 == entry.v.0) {
            problems.push(error!(entry.span, "duplicate argument"));
        } else {
            self.map.push(entry);
        }
    }

    /// Add multiple spanned key-value pairs.
    pub fn extend<I>(&mut self, problems: &mut Problems, items: I)
    where I: IntoIterator<Item=Spanned<(K, V)>> {
        for item in items.into_iter() {
            self.insert(problems, item);
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
    /// values. When the mapping introduces new duplicates, problems are
    /// generated.
    pub fn dedup<F, K2, V2>(&self, problems: &mut Problems, mut f: F) -> DedupMap<K2, V2>
    where F: FnMut(&K, &V) -> (K2, V2), K2: Eq {
        let mut map = DedupMap::new();

        for Spanned { v: (key, value), span } in self.map.iter() {
            let (key, value) = f(key, value);
            map.insert(problems, Spanned { v: (key, value), span: *span });
        }

        map
    }

    /// Iterate over the (key, value) pairs.
    pub fn iter(&self) -> impl Iterator<Item=&(K, V)> {
        self.map.iter().map(|e| &e.v)
    }
}

/// A map for storing a value for axes given by keyword arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisMap<V>(DedupMap<AxisKey, V>);

impl<V: Value> AxisMap<V> {
    /// Parse an axis map from the object.
    pub fn parse<K>(
        problems: &mut Problems,
        object: &mut Object,
    ) -> AxisMap<V> where K: Key + Into<AxisKey> {
        let values: Vec<_> = object
            .get_all_spanned::<K, V>(problems)
            .map(|s| s.map(|(k, v)| (k.into(), v)))
            .collect();

        AxisMap(DedupMap::from_iter(problems, values))
    }

    /// Deduplicate from specific or generic to just specific axes.
    pub fn dedup(&self, problems: &mut Problems, axes: LayoutAxes) -> DedupMap<SpecificAxis, V>
    where V: Clone {
        self.0.dedup(problems, |key, val| (key.to_specific(axes), val.clone()))
    }
}

/// A map for storing values for axes that are given through a combination of
/// (two) positional and keyword arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct PosAxisMap<V>(DedupMap<PosAxisKey, V>);

impl<V: Value> PosAxisMap<V> {
    /// Parse a positional/axis map from the function arguments.
    pub fn parse<K>(
        problems: &mut Problems,
        args: &mut FuncArgs,
    ) -> PosAxisMap<V> where K: Key + Into<AxisKey> {
        let mut map = DedupMap::new();

        for &key in &[PosAxisKey::First, PosAxisKey::Second] {
            if let Some(Spanned { v, span }) = args.pos.get::<Spanned<V>>(problems) {
                map.insert(problems, Spanned { v: (key, v), span })
            }
        }

        let keywords: Vec<_> = args.key
            .get_all_spanned::<K, V>(problems)
            .map(|s| s.map(|(k, v)| (PosAxisKey::Keyword(k.into()), v)))
            .collect();

        map.extend(problems, keywords);

        PosAxisMap(map)
    }

    /// Deduplicate from positional arguments and keyword arguments for generic
    /// or specific axes to just generic axes.
    pub fn dedup<F>(
        &self,
        problems: &mut Problems,
        axes: LayoutAxes,
        mut f: F,
    ) -> DedupMap<GenericAxis, V>
    where
        F: FnMut(&V) -> Option<GenericAxis>,
        V: Clone,
    {
        self.0.dedup(problems, |key, val| {
            (match key {
                PosAxisKey::First => f(val).unwrap_or(GenericAxis::Primary),
                PosAxisKey::Second => f(val).unwrap_or(GenericAxis::Secondary),
                PosAxisKey::Keyword(AxisKey::Specific(axis)) => axis.to_generic(axes),
                PosAxisKey::Keyword(AxisKey::Generic(axis)) => *axis,
            }, val.clone())
        })
    }
}

/// A map for storing padding given for a combination of all sides, opposing
/// sides or single sides.
#[derive(Debug, Clone, PartialEq)]
pub struct PaddingMap(DedupMap<PaddingKey<AxisKey>, Option<PSize>>);

impl PaddingMap {
    /// Parse a padding map from the function arguments.
    pub fn parse(problems: &mut Problems, args: &mut FuncArgs) -> PaddingMap {
        let mut map = DedupMap::new();

        let all = args.pos.get::<Spanned<Defaultable<PSize>>>(problems);
        if let Some(Spanned { v, span }) = all {
            map.insert(problems, Spanned { v: (PaddingKey::All, v.into()), span });
        }

        let paddings: Vec<_> = args.key
            .get_all_spanned::<PaddingKey<AxisKey>, Defaultable<PSize>>(problems)
            .map(|s| s.map(|(k, v)| (k, v.into())))
            .collect();

        map.extend(problems, paddings);

        PaddingMap(map)
    }

    /// Apply the specified padding on a value box of optional, scalable sizes.
    pub fn apply(
        &self,
        problems: &mut Problems,
        axes: LayoutAxes,
        padding: &mut ValueBox<Option<PSize>>
    ) {
        use PaddingKey::*;

        let map = self.0.dedup(problems, |key, &val| {
            (match key {
                All => All,
                Both(axis) => Both(axis.to_specific(axes)),
                Side(axis, alignment) => {
                    let generic = axis.to_generic(axes);
                    let axis = axis.to_specific(axes);
                    Side(axis, alignment.to_specific(axes, generic))
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
