use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::{Add, AddAssign};
use std::sync::Arc;

use ecow::{eco_format, EcoString};
use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{array, func, scope, ty, Array, Repr, Str, Value};
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

/// A map from string keys to values.
///
/// You can construct a dictionary by enclosing comma-separated `key: value`
/// pairs in parentheses. The values do not have to be of the same type. Since
/// empty parentheses already yield an empty array, you have to use the special
/// `(:)` syntax to create an empty dictionary.
///
/// A dictionary is conceptually similar to an array, but it is indexed by
/// strings instead of integers. You can access and create dictionary entries
/// with the `.at()` method. If you know the key statically, you can
/// alternatively use [field access notation]($scripting/#fields) (`.key`) to
/// access the value. Dictionaries can be added with the `+` operator and
/// [joined together]($scripting/#blocks). To check whether a key is present in
/// the dictionary, use the `in` keyword.
///
/// You can iterate over the pairs in a dictionary using a [for
/// loop]($scripting/#loops). This will iterate in the order the pairs were
/// inserted / declared.
///
/// # Example
/// ```example
/// #let dict = (
///   name: "Typst",
///   born: 2019,
/// )
///
/// #dict.name \
/// #(dict.launch = 20)
/// #dict.len() \
/// #dict.keys() \
/// #dict.values() \
/// #dict.at("born") \
/// #dict.insert("city", "Berlin ")
/// #("name" in dict)
/// ```
#[ty(scope, name = "dictionary")]
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

    /// Borrow the value at the given key.
    pub fn get(&self, key: &str) -> StrResult<&Value> {
        self.0.get(key).ok_or_else(|| missing_key(key))
    }

    /// Mutably borrow the value the given `key` maps to.
    pub fn at_mut(&mut self, key: &str) -> StrResult<&mut Value> {
        Arc::make_mut(&mut self.0)
            .get_mut(key)
            .ok_or_else(|| missing_key_no_default(key))
    }

    /// Remove the value if the dictionary contains the given key.
    pub fn take(&mut self, key: &str) -> StrResult<Value> {
        Arc::make_mut(&mut self.0).remove(key).ok_or_else(|| missing_key(key))
    }

    /// Whether the dictionary contains a specific key.
    pub fn contains(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    /// Clear the dictionary.
    pub fn clear(&mut self) {
        if Arc::strong_count(&self.0) == 1 {
            Arc::make_mut(&mut self.0).clear();
        } else {
            *self = Self::new();
        }
    }

    /// Iterate over pairs of references to the contained keys and values.
    pub fn iter(&self) -> indexmap::map::Iter<Str, Value> {
        self.0.iter()
    }

    /// Return an "unexpected key" error if there is any remaining pair.
    pub fn finish(&self, expected: &[&str]) -> StrResult<()> {
        if let Some((key, _)) = self.iter().next() {
            let parts: Vec<_> = expected.iter().map(|s| eco_format!("\"{s}\"")).collect();
            let mut msg = format!("unexpected key {}, valid keys are ", key.repr());
            msg.push_str(&separated_list(&parts, "and"));
            return Err(msg.into());
        }
        Ok(())
    }
}

#[scope]
impl Dict {
    /// The number of pairs in the dictionary.
    #[func(title = "Length")]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns the value associated with the specified key in the dictionary.
    /// May be used on the left-hand side of an assignment if the key is already
    /// present in the dictionary. Returns the default value if the key is not
    /// part of the dictionary or fails with an error if no default value was
    /// specified.
    #[func]
    pub fn at(
        &self,
        /// The key at which to retrieve the item.
        key: Str,
        /// A default value to return if the key is not part of the dictionary.
        #[named]
        default: Option<Value>,
    ) -> StrResult<Value> {
        self.0
            .get(&key)
            .cloned()
            .or(default)
            .ok_or_else(|| missing_key_no_default(&key))
    }

    /// Inserts a new pair into the dictionary and return the value. If the
    /// dictionary already contains this key, the value is updated.
    #[func]
    pub fn insert(
        &mut self,
        /// The key of the pair that should be inserted.
        key: Str,
        /// The value of the pair that should be inserted.
        value: Value,
    ) {
        Arc::make_mut(&mut self.0).insert(key, value);
    }

    /// Removes a pair from the dictionary by key and return the value.
    #[func]
    pub fn remove(
        &mut self,
        key: Str,
        /// A default value to return if the key does not exist.
        #[named]
        default: Option<Value>,
    ) -> StrResult<Value> {
        Arc::make_mut(&mut self.0)
            .shift_remove(&key)
            .or(default)
            .ok_or_else(|| missing_key(&key))
    }

    /// Returns the keys of the dictionary as an array in insertion order.
    #[func]
    pub fn keys(&self) -> Array {
        self.0.keys().cloned().map(Value::Str).collect()
    }

    /// Returns the values of the dictionary as an array in insertion order.
    #[func]
    pub fn values(&self) -> Array {
        self.0.values().cloned().collect()
    }

    /// Returns the keys and values of the dictionary as an array of pairs. Each
    /// pair is represented as an array of length two.
    #[func]
    pub fn pairs(&self) -> Array {
        self.0
            .iter()
            .map(|(k, v)| Value::Array(array![k.clone(), v.clone()]))
            .collect()
    }
}

impl Debug for Dict {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.0.iter()).finish()
    }
}

impl Repr for Dict {
    fn repr(&self) -> EcoString {
        if self.is_empty() {
            return "(:)".into();
        }

        let max = 40;
        let mut pieces: Vec<_> = self
            .iter()
            .take(max)
            .map(|(key, value)| {
                if is_ident(key) {
                    eco_format!("{key}: {}", value.repr())
                } else {
                    eco_format!("{}: {}", key.repr(), value.repr())
                }
            })
            .collect();

        if self.len() > max {
            pieces.push(eco_format!(".. ({} pairs omitted)", self.len() - max));
        }

        pretty_array_like(&pieces, false).into()
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

impl<'de> Deserialize<'de> for Dict {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(IndexMap::<Str, Value>::deserialize(deserializer)?.into())
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
    eco_format!("dictionary does not contain key {}", key.repr())
}

/// The missing key access error message when no default was fiven.
#[cold]
fn missing_key_no_default(key: &str) -> EcoString {
    eco_format!(
        "dictionary does not contain key {} \
         and no default value was specified",
        key.repr()
    )
}
