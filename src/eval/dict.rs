use std::collections::BTreeMap;
use std::fmt::{self, Debug, Formatter, Write};
use std::ops::{Add, AddAssign};
use std::sync::Arc;

use super::{Args, Array, Func, Value};
use crate::diag::{StrResult, TypResult};
use crate::syntax::Spanned;
use crate::util::{ArcExt, EcoString};
use crate::Context;

/// Create a new [`Dict`] from key-value pairs.
#[allow(unused_macros)]
macro_rules! dict {
    ($($key:expr => $value:expr),* $(,)?) => {{
        #[allow(unused_mut)]
        let mut map = std::collections::BTreeMap::new();
        $(map.insert($key.into(), $value.into());)*
        $crate::eval::Dict::from_map(map)
    }};
}

/// A dictionary from strings to values with clone-on-write value semantics.
#[derive(Default, Clone, PartialEq, Hash)]
pub struct Dict(Arc<BTreeMap<EcoString, Value>>);

impl Dict {
    /// Create a new, empty dictionary.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new dictionary from a mapping of strings to values.
    pub fn from_map(map: BTreeMap<EcoString, Value>) -> Self {
        Self(Arc::new(map))
    }

    /// Whether the dictionary is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The number of pairs in the dictionary.
    pub fn len(&self) -> i64 {
        self.0.len() as i64
    }

    /// Borrow the value the given `key` maps to.
    pub fn get(&self, key: &EcoString) -> StrResult<&Value> {
        self.0.get(key).ok_or_else(|| missing_key(key))
    }

    /// Mutably borrow the value the given `key` maps to.
    ///
    /// This inserts the key with [`None`](Value::None) as the value if not
    /// present so far.
    pub fn get_mut(&mut self, key: EcoString) -> &mut Value {
        Arc::make_mut(&mut self.0).entry(key).or_default()
    }

    /// Whether the dictionary contains a specific key.
    pub fn contains(&self, key: &EcoString) -> bool {
        self.0.contains_key(key)
    }

    /// Insert a mapping from the given `key` to the given `value`.
    pub fn insert(&mut self, key: EcoString, value: Value) {
        Arc::make_mut(&mut self.0).insert(key, value);
    }

    /// Remove a mapping by `key`.
    pub fn remove(&mut self, key: &EcoString) -> StrResult<()> {
        match Arc::make_mut(&mut self.0).remove(key) {
            Some(_) => Ok(()),
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
        self.iter().map(|(key, _)| Value::Str(key.clone())).collect()
    }

    /// Return the values of the dictionary as an array.
    pub fn values(&self) -> Array {
        self.iter().map(|(_, value)| value.clone()).collect()
    }

    /// Transform each pair in the array with a function.
    pub fn map(&self, ctx: &mut Context, f: Spanned<Func>) -> TypResult<Array> {
        Ok(self
            .iter()
            .map(|(key, value)| {
                f.v.call(
                    ctx,
                    Args::from_values(f.span, [Value::Str(key.clone()), value.clone()]),
                )
            })
            .collect::<TypResult<_>>()?)
    }

    /// Iterate over pairs of references to the contained keys and values.
    pub fn iter(&self) -> std::collections::btree_map::Iter<EcoString, Value> {
        self.0.iter()
    }
}

/// The missing key access error message.
#[cold]
fn missing_key(key: &EcoString) -> String {
    format!("dictionary does not contain key: {:?}", key)
}

impl Debug for Dict {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_char('(')?;
        if self.is_empty() {
            f.write_char(':')?;
        }
        for (i, (key, value)) in self.iter().enumerate() {
            f.write_str(key)?;
            f.write_str(": ")?;
            value.fmt(f)?;
            if i + 1 < self.0.len() {
                f.write_str(", ")?;
            }
        }
        f.write_char(')')
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

impl Extend<(EcoString, Value)> for Dict {
    fn extend<T: IntoIterator<Item = (EcoString, Value)>>(&mut self, iter: T) {
        Arc::make_mut(&mut self.0).extend(iter);
    }
}

impl FromIterator<(EcoString, Value)> for Dict {
    fn from_iter<T: IntoIterator<Item = (EcoString, Value)>>(iter: T) -> Self {
        Self(Arc::new(iter.into_iter().collect()))
    }
}

impl IntoIterator for Dict {
    type Item = (EcoString, Value);
    type IntoIter = std::collections::btree_map::IntoIter<EcoString, Value>;

    fn into_iter(self) -> Self::IntoIter {
        Arc::take(self.0).into_iter()
    }
}

impl<'a> IntoIterator for &'a Dict {
    type Item = (&'a EcoString, &'a Value);
    type IntoIter = std::collections::btree_map::Iter<'a, EcoString, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
