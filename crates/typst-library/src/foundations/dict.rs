use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::{Add, AddAssign};
use std::sync::Arc;

use ecow::{eco_format, EcoString};
use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use typst_syntax::is_ident;
use typst_utils::ArcExt;

use crate::diag::{Hint, HintedStrResult, StrResult};
use crate::foundations::{
    array, cast, func, repr, scope, ty, Array, Module, Repr, Str, Value,
};

/// Create a new [`Dict`] from key-value pairs.
#[macro_export]
#[doc(hidden)]
macro_rules! __dict {
    ($($key:expr => $value:expr),* $(,)?) => {{
        #[allow(unused_mut)]
        let mut map = $crate::foundations::IndexMap::new();
        $(map.insert($key.into(), $crate::foundations::IntoValue::into_value($value));)*
        $crate::foundations::Dict::from(map)
    }};
}

#[doc(inline)]
pub use crate::__dict as dict;

/// A key that can be used to get a dictionary: either the index of a positional
/// argument, or the name of a named argument.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DictionaryKey {
    Index(i64),
    Name(Str),
}

cast! {
    DictionaryKey,
    v: i64 => Self::Index(v),
    v: Str => Self::Name(v),
}

impl Repr for DictionaryKey {
    fn repr(&self) -> EcoString {
        match self {
            DictionaryKey::Index(i) => eco_format!("[{}]", i),
            DictionaryKey::Name(name) => eco_format!("{}", name),
        }
    }
}

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
/// #dict.at(1) \
/// #dict.insert("city", "Berlin ")
/// #("name" in dict)
/// ```
#[ty(scope, cast, name = "dictionary")]
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
    pub fn get(&self, key: &DictionaryKey) -> StrResult<&Value> {
        let item = match key {
            DictionaryKey::Index(i) => {
                self.0.get_index(*i as usize).map(|(_, item)| item)
            }
            DictionaryKey::Name(name) => self.0.get(name),
        };
        item.ok_or_else(|| missing_key(&key.repr()))
    }

    /// Mutably borrow the value the given `key` maps to.
    pub fn at_mut(&mut self, key: &str) -> HintedStrResult<&mut Value> {
        Arc::make_mut(&mut self.0)
            .get_mut(key)
            .ok_or_else(|| missing_key(key))
            .hint("use `insert` to add or update values")
    }

    /// Remove the value if the dictionary contains the given key.
    pub fn take(&mut self, key: &str) -> StrResult<Value> {
        Arc::make_mut(&mut self.0)
            .shift_remove(key)
            .ok_or_else(|| missing_key(key))
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

    /// Check if there is any remaining pair, and if so return an
    /// "unexpected key" error.
    pub fn finish(&self, expected: &[&str]) -> StrResult<()> {
        let mut iter = self.iter().peekable();
        if iter.peek().is_none() {
            return Ok(());
        }
        let unexpected: Vec<&str> = iter.map(|kv| kv.0.as_str()).collect();

        Err(Self::unexpected_keys(unexpected, Some(expected)))
    }

    // Return an "unexpected key" error string.
    pub fn unexpected_keys(
        unexpected: Vec<&str>,
        hint_expected: Option<&[&str]>,
    ) -> EcoString {
        let format_as_list = |arr: &[&str]| {
            repr::separated_list(
                &arr.iter().map(|s| eco_format!("\"{s}\"")).collect::<Vec<_>>(),
                "and",
            )
        };

        let mut msg = String::from(match unexpected.len() {
            1 => "unexpected key ",
            _ => "unexpected keys ",
        });

        msg.push_str(&format_as_list(&unexpected[..]));

        if let Some(expected) = hint_expected {
            msg.push_str(", valid keys are ");
            msg.push_str(&format_as_list(expected));
        }

        msg.into()
    }
}

#[scope]
impl Dict {
    /// Converts a value into a dictionary.
    ///
    /// Note that this function is only intended for conversion of a
    /// dictionary-like value to a dictionary, not for creation of a dictionary
    /// from individual pairs. Use the dictionary syntax `(key: value)` instead.
    ///
    /// ```example
    /// #dictionary(sys).at("version")
    /// ```
    #[func(constructor)]
    pub fn construct(
        /// The value that should be converted to a dictionary.
        value: ToDict,
    ) -> Dict {
        value.0
    }

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
        key: DictionaryKey,
        /// A default value to return if the key is not part of the dictionary.
        #[named]
        default: Option<Value>,
    ) -> StrResult<Value> {
        match self.get(&key) {
            Ok(value) => Ok(value.clone()),
            Err(_) => default.ok_or_else(|| missing_key_no_default(&key.repr())),
        }
    }

    /// Inserts a new pair into the dictionary. If the dictionary already
    /// contains this key, the value is updated.
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
        /// The key of the pair to remove.
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

/// A value that can be cast to dictionary.
pub struct ToDict(Dict);

cast! {
    ToDict,
    v: Module => Self(v.scope().iter().map(|(k, v, _)| (Str::from(k.clone()), v.clone())).collect()),
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

        repr::pretty_array_like(&pieces, false).into()
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

/// The missing key access error message when no default was given.
#[cold]
fn missing_key_no_default(key: &str) -> EcoString {
    eco_format!(
        "dictionary does not contain key {} \
         and no default value was specified",
        key.repr()
    )
}
