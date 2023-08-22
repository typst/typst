use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::{Add, AddAssign};
use std::sync::Arc;

use ecow::{eco_format, EcoString};
use serde::{Serialize, Serializer};

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
        let mut map = $crate::eval::IndexMap::new();
        $(map.insert($key.into(), $crate::eval::IntoValue::into_value($value));)*
        $crate::eval::Dict::from(map)
    }};
}

#[doc(inline)]
pub use crate::__dict as dict;

#[doc(inline)]
pub use indexmap::IndexMap;

/// A reference-counted dictionary with value semantics.
#[derive(Default, Clone, PartialEq)]
pub struct Dict(Arc<IndexMap<Str, Value>>);

impl Dict {
    /// Create a new, empty dictionary.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the dictionary is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The number of pairs in the dictionary.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Borrow the value the given `key` maps to,
    pub fn at(&self, key: &str, default: Option<Value>) -> StrResult<Value> {
        self.0
            .get(key)
            .cloned()
            .or(default)
            .ok_or_else(|| missing_key_no_default(key))
    }

    /// Mutably borrow the value the given `key` maps to.
    pub fn at_mut(&mut self, key: &str) -> StrResult<&mut Value> {
        Arc::make_mut(&mut self.0)
            .get_mut(key)
            .ok_or_else(|| missing_key_no_default(key))
    }

    /// Remove the value if the dictionary contains the given key.
    pub fn take(&mut self, key: &str) -> StrResult<Value> {
        Arc::make_mut(&mut self.0)
            .remove(key)
            .ok_or_else(|| eco_format!("missing key: {:?}", Str::from(key)))
    }

    /// Whether the dictionary contains a specific key.
    pub fn contains(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    /// Insert a mapping from the given `key` to the given `value`.
    pub fn insert(&mut self, key: Str, value: Value) {
        Arc::make_mut(&mut self.0).insert(key, value);
    }

    /// Remove a mapping by `key` and return the value.
    pub fn remove(&mut self, key: &str) -> StrResult<Value> {
        match Arc::make_mut(&mut self.0).shift_remove(key) {
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
    pub fn keys(&self) -> Array {
        self.0.keys().cloned().map(Value::Str).collect()
    }

    /// Return the values of the dictionary as an array.
    pub fn values(&self) -> Array {
        self.0.values().cloned().collect()
    }

    /// Return the values of the dictionary as an array of pairs (arrays of
    /// length two).
    pub fn pairs(&self) -> Array {
        self.0
            .iter()
            .map(|(k, v)| Value::Array(array![k.clone(), v.clone()]))
            .collect()
    }

    /// Iterate over pairs of references to the contained keys and values.
    pub fn iter(&self) -> indexmap::map::Iter<Str, Value> {
        self.0.iter()
    }

    /// Return an "unexpected key" error if there is any remaining pair.
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
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.is_empty() {
            return f.write_str("(:)");
        }

        let max = 40;
        let mut pieces: Vec<_> = self
            .iter()
            .take(max)
            .map(|(key, value)| {
                if is_ident(key) {
                    eco_format!("{key}: {value:?}")
                } else {
                    eco_format!("{key:?}: {value:?}")
                }
            })
            .collect();

        if self.len() > max {
            pieces.push(eco_format!(".. ({} pairs omitted)", self.len() - max));
        }

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

impl Hash for Dict {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.0.len());
        for item in self {
            item.hash(state);
        }
    }
}

impl Serialize for Dict {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
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
    type IntoIter = indexmap::map::IntoIter<Str, Value>;

    fn into_iter(self) -> Self::IntoIter {
        Arc::take(self.0).into_iter()
    }
}

impl<'a> IntoIterator for &'a Dict {
    type Item = (&'a Str, &'a Value);
    type IntoIter = indexmap::map::Iter<'a, Str, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl From<IndexMap<Str, Value>> for Dict {
    fn from(map: IndexMap<Str, Value>) -> Self {
        Self(Arc::new(map))
    }
}

/// The missing key access error message.
#[cold]
fn missing_key(key: &str) -> EcoString {
    eco_format!("dictionary does not contain key {:?}", Str::from(key))
}

/// The missing key access error message when no default was fiven.
#[cold]
fn missing_key_no_default(key: &str) -> EcoString {
    eco_format!(
        "dictionary does not contain key {:?} \
         and no default value was specified",
        Str::from(key)
    )
}
