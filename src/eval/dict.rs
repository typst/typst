use std::collections::BTreeMap;
use std::fmt::{self, Debug, Formatter};
use std::ops::{Add, AddAssign};
use std::sync::Arc;

use ecow::{eco_format, EcoString};

use super::{array, Array, Str, Value};
use crate::diag::StrResult;
use crate::syntax::is_ident;
use crate::util::{pretty_array_like, separated_list, ArcExt};

/// Create a new [`Dict`] from key-value pairs.
#[macro_export]
#[doc(hidden)]
macro_rules! __dict {
    ($($key:expr => $value:expr),* $(,)?) => {{
        #[allow(unused_mut)]
        let mut map = std::collections::BTreeMap::new();
        $(map.insert($key.into(), $value.into());)*
        $crate::eval::Dict::from_map(map)
    }};
}

#[doc(inline)]
pub use crate::__dict as dict;

/// A reference-counted dictionary with value semantics.
#[derive(Default, Clone, PartialEq, Hash)]
pub struct Dict(Arc<BTreeMap<Str, Value>>);

impl Dict {
    /// Create a new, empty dictionary.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new dictionary from a mapping of strings to values.
    #[must_use]
    pub fn from_map(map: BTreeMap<Str, Value>) -> Self {
        Self(Arc::new(map))
    }

    /// Whether the dictionary is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The number of pairs in the dictionary.
    #[must_use]
    pub fn len(&self) -> i64 {
        self.0.len() as i64
    }

    /// Borrow the value the given `key` maps to.
    ///
    /// # Errors
    ///
    /// If the key is missing.
    pub fn at(&self, key: &str) -> StrResult<&Value> {
        self.0.get(key).ok_or_else(|| missing_key(key))
    }

    /// Mutably borrow the value the given `key` maps to.
    ///
    /// # Errors
    ///
    /// If the key is missing.
    pub fn at_mut(&mut self, key: &str) -> StrResult<&mut Value> {
        Arc::make_mut(&mut self.0)
            .get_mut(key)
            .ok_or_else(|| missing_key(key))
    }

    /// Remove the value if the dictionary contains the given key.
    ///
    /// # Errors
    ///
    /// If the key is missing.
    pub fn take(&mut self, key: &str) -> StrResult<Value> {
        Arc::make_mut(&mut self.0)
            .remove(key)
            .ok_or_else(|| eco_format!("missing key: {:?}", Str::from(key)))
    }

    /// Whether the dictionary contains a specific key.
    #[must_use]
    pub fn contains(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    /// Insert a mapping from the given `key` to the given `value`.
    pub fn insert(&mut self, key: Str, value: Value) {
        Arc::make_mut(&mut self.0).insert(key, value);
    }

    /// Remove a mapping by `key` and return the value.
    ///
    /// # Errors
    ///
    /// If the key is missing.
    pub fn remove(&mut self, key: &str) -> StrResult<Value> {
        match Arc::make_mut(&mut self.0).remove(key) {
            Some(value) => Ok(value),
            None => Err(missing_key(key)),
        }
    }

    /// Clear the dictionary.
    pub fn clear(&mut self) {
        if Arc::strong_count(&self.0) == 1 {
            Arc::make_mut(&mut self.0).clear();
        } else {
            *self = Self::new();
        }
    }

    /// Return the keys of the dictionary as an array.
    #[must_use]
    pub fn keys(&self) -> Array {
        self.0.keys().cloned().map(Value::Str).collect()
    }

    /// Return the values of the dictionary as an array.
    #[must_use]
    pub fn values(&self) -> Array {
        self.0.values().cloned().collect()
    }

    /// Return the values of the dictionary as an array of pairs (arrays of
    /// length two).
    #[must_use]
    pub fn pairs(&self) -> Array {
        self.0
            .iter()
            .map(|(k, v)| Value::Array(array![k.clone(), v.clone()]))
            .collect()
    }

    /// Iterate over pairs of references to the contained keys and values.
    pub fn iter(&self) -> std::collections::btree_map::Iter<'_, Str, Value> {
        self.0.iter()
    }

    /// Return an "unexpected key" error if there is any remaining pair.
    #[allow(clippy::missing_errors_doc /* false positive */)]
    pub fn finish(&self, expected: &[&str]) -> StrResult<()> {
        if let Some((key, _)) = self.iter().next() {
            let parts: Vec<_> = expected.iter().map(|s| eco_format!("\"{s}\"")).collect();
            let mut msg = format!("unexpected key {key:?}, valid keys are ");
            msg.push_str(&separated_list(&parts, "and"));
            return Err(msg.into());
        }
        Ok(())
    }
}

impl Debug for Dict {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return f.write_str("(:)");
        }

        let pieces: Vec<_> = self
            .iter()
            .map(|(key, value)| {
                if is_ident(key) {
                    eco_format!("{key}: {value:?}")
                } else {
                    eco_format!("{key:?}: {value:?}")
                }
            })
            .collect();

        f.write_str(&pretty_array_like(&pieces, false))
    }
}

impl Add for Dict {
    type Output = Self;

    fn add(mut self, rhs: Dict) -> Self::Output {
        self += rhs;
        self
    }
}

impl AddAssign for Dict {
    fn add_assign(&mut self, rhs: Dict) {
        match Arc::try_unwrap(rhs.0) {
            Ok(map) => self.extend(map),
            Err(rc) => self.extend(rc.iter().map(|(k, v)| (k.clone(), v.clone()))),
        }
    }
}

impl Extend<(Str, Value)> for Dict {
    fn extend<T: IntoIterator<Item = (Str, Value)>>(&mut self, iter: T) {
        Arc::make_mut(&mut self.0).extend(iter);
    }
}

impl FromIterator<(Str, Value)> for Dict {
    fn from_iter<T: IntoIterator<Item = (Str, Value)>>(iter: T) -> Self {
        Self(Arc::new(iter.into_iter().collect()))
    }
}

impl IntoIterator for Dict {
    type Item = (Str, Value);
    type IntoIter = std::collections::btree_map::IntoIter<Str, Value>;

    fn into_iter(self) -> Self::IntoIter {
        Arc::take(self.0).into_iter()
    }
}

impl<'a> IntoIterator for &'a Dict {
    type Item = (&'a Str, &'a Value);
    type IntoIter = std::collections::btree_map::Iter<'a, Str, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// The missing key access error message.
#[cold]
#[must_use]
fn missing_key(key: &str) -> EcoString {
    eco_format!("dictionary does not contain key {:?}", Str::from(key))
}
