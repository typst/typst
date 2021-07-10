use std::collections::BTreeMap;
use std::fmt::{self, Debug, Formatter};
use std::iter::FromIterator;
use std::ops::{Add, AddAssign};
use std::rc::Rc;

use super::Value;
use crate::eco::EcoString;

/// Create a new [`Dict`] from key-value pairs.
#[macro_export]
macro_rules! dict {
    ($($key:expr => $value:expr),* $(,)?) => {{
        #[allow(unused_mut)]
        let mut map = std::collections::BTreeMap::new();
        $(map.insert($crate::eco::EcoString::from($key), $crate::eval::Value::from($value));)*
        $crate::eval::Dict::from_map(map)
    }};
}

/// A variably-typed dictionary with clone-on-write value semantics.
#[derive(Clone, PartialEq)]
pub struct Dict {
    map: Rc<BTreeMap<EcoString, Value>>,
}

impl Dict {
    /// Create a new, empty dictionary.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new dictionary from a mapping of strings to values.
    pub fn from_map(map: BTreeMap<EcoString, Value>) -> Self {
        Self { map: Rc::new(map) }
    }

    /// Whether the dictionary is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The number of pairs in the dictionary.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Borrow the value the given `key` maps to.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.map.get(key)
    }

    /// Mutably borrow the value the given `key` maps to.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        Rc::make_mut(&mut self.map).get_mut(key)
    }

    /// Insert a mapping from the given `key` to the given `value`.
    pub fn insert(&mut self, key: EcoString, value: Value) {
        Rc::make_mut(&mut self.map).insert(key, value);
    }

    /// Extend the dictionary with the values from another dictionary.
    pub fn extend(&mut self, other: &Dict) {
        Rc::make_mut(&mut self.map).extend(other.into_iter())
    }

    /// Clear the dictionary.
    pub fn clear(&mut self) {
        if Rc::strong_count(&mut self.map) == 1 {
            Rc::make_mut(&mut self.map).clear();
        } else {
            *self = Self::new();
        }
    }

    /// Iterate over pairs of the contained keys and values.
    pub fn into_iter(&self) -> impl Iterator<Item = (EcoString, Value)> + Clone + '_ {
        // TODO: Actually consume the map if the ref-count is 1?
        self.iter().map(|(k, v)| (k.clone(), v.clone()))
    }

    /// Iterate over pairs of references to the contained keys and values.
    pub fn iter(&self) -> std::collections::btree_map::Iter<EcoString, Value> {
        self.map.iter()
    }
}

impl Default for Dict {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for Dict {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_map().entries(self.map.iter()).finish()
    }
}

impl FromIterator<(EcoString, Value)> for Dict {
    fn from_iter<T: IntoIterator<Item = (EcoString, Value)>>(iter: T) -> Self {
        Dict { map: Rc::new(iter.into_iter().collect()) }
    }
}

impl<'a> IntoIterator for &'a Dict {
    type Item = (&'a EcoString, &'a Value);
    type IntoIter = std::collections::btree_map::Iter<'a, EcoString, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl Add<&Dict> for Dict {
    type Output = Self;

    fn add(mut self, rhs: &Dict) -> Self::Output {
        self.extend(rhs);
        self
    }
}

impl AddAssign<&Dict> for Dict {
    fn add_assign(&mut self, rhs: &Dict) {
        self.extend(rhs);
    }
}
