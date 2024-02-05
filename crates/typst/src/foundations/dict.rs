use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::{Add, AddAssign};
use std::sync::Arc;

use ecow::{eco_format, EcoString};
use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::diag::StrResult;
use crate::foundations::{
    array, func, repr, scope, ty, Array, CastInfo, FromValue, IntoValue, Reflect, Repr,
    Str, Value,
};
use crate::syntax::is_ident;
use crate::util::ArcExt;

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

/// A map from string keys to values.
///
/// You can construct a dictionary by enclosing comma-separated `key: value`
/// pairs in parentheses. The keys can be strings or integers. The values do
/// not have to be of the same type. Since an empty parenthesis pair yields an
/// empty array, you have to use the special `(:)` syntax to create an empty
/// dictionary.
///
/// You can access and create dictionary entries with the `.at()` method. If you
/// know the key string statically, you can alternatively use [field access
/// notation]($scripting/#fields) (`.key`) to access the value. Dictionaries can
/// be added with the `+` operator and [joined together]($scripting/#blocks). To
/// check whether a key is present in the dictionary, use the `in` keyword.
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
#[ty(scope, cast, name = "dictionary")]
#[derive(Default, Clone, PartialEq)]
pub struct Dict(Arc<IndexMap<DictKey, Value>>);

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub enum DictKey {
    Str(Str),
    Int(i64),
}

impl From<&str> for DictKey {
    fn from(v: &str) -> Self {
        DictKey::Str(v.into())
    }
}

impl From<Str> for DictKey {
    fn from(v: Str) -> Self {
        DictKey::Str(v)
    }
}

impl From<&Str> for DictKey {
    fn from(v: &Str) -> Self {
        DictKey::Str(v.clone())
    }
}

impl From<EcoString> for DictKey {
    fn from(v: EcoString) -> Self {
        DictKey::Str(v.into())
    }
}

impl From<&EcoString> for DictKey {
    fn from(v: &EcoString) -> Self {
        DictKey::Str(v.clone().into())
    }
}

impl From<i64> for DictKey {
    fn from(v: i64) -> Self {
        DictKey::Int(v)
    }
}

impl From<DictKey> for EcoString {
    fn from(v: DictKey) -> Self {
        match v {
            DictKey::Str(s) => s.into(),
            DictKey::Int(i) => i.to_string().into(),
        }
    }
}

impl FromValue for DictKey {
    fn from_value(value: Value) -> StrResult<Self> {
        match value {
            Value::Str(s) => Ok(Self::Str(s)),
            Value::Int(i) => Ok(Self::Int(i)),
            _ => Err(Self::error(&value)),
        }
    }
}

impl IntoValue for DictKey {
    fn into_value(self) -> Value {
        match self {
            Self::Str(s) => Value::Str(s),
            Self::Int(i) => Value::Int(i),
        }
    }
}

impl Reflect for DictKey {
    fn input() -> CastInfo {
        Str::input() + i64::input()
    }

    fn output() -> CastInfo {
        Str::output() + i64::output()
    }

    fn castable(_: &Value) -> bool {
        true
    }
}

impl Repr for DictKey {
    fn repr(&self) -> EcoString {
        match self {
            Self::Str(s) => Value::Str(s.clone()).repr(),
            Self::Int(i) => Value::Int(*i).repr(),
        }
    }
}

impl Display for DictKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DictKey::Str(s) if is_ident(s) => Display::fmt(s, f),
            DictKey::Str(s) => write!(f, "\"{s}\""),
            DictKey::Int(i) => write!(f, "{i}"),
        }
    }
}

/// Create a new [`Vec`] of [`DictKey`].
#[macro_export]
#[doc(hidden)]
macro_rules! __dict_keys {
    ($($value:expr),* $(,)?) => {
        vec![$($crate::foundations::DictKey::from($value)),*]
    };
}

#[doc(inline)]
pub use crate::__dict_keys as dict_keys;

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
    pub fn get(&self, key: impl Into<DictKey>) -> StrResult<&Value> {
        let key: DictKey = key.into();
        self.0.get(&key).ok_or_else(|| missing_key(&key))
    }

    /// Mutably borrow the value the given `key` maps to.
    pub fn at_mut(&mut self, key: impl Into<DictKey>) -> StrResult<&mut Value> {
        let key: DictKey = key.into();
        Arc::make_mut(&mut self.0)
            .get_mut(&key)
            .ok_or_else(|| missing_key_no_default(&key))
    }

    /// Remove the value if the dictionary contains the given key.
    pub fn take(&mut self, key: impl Into<DictKey>) -> StrResult<Value> {
        let key: DictKey = key.into();
        Arc::make_mut(&mut self.0)
            .remove(&key)
            .ok_or_else(|| missing_key(&key))
    }

    /// Whether the dictionary contains a specific key.
    pub fn contains(&self, key: impl Into<DictKey>) -> bool {
        let key: DictKey = key.into();
        self.0.contains_key(&key)
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
    pub fn iter(&self) -> indexmap::map::Iter<DictKey, Value> {
        self.0.iter()
    }

    /// Check if there is any remaining pair, and if so return an
    /// "unexpected key" error.
    pub fn finish(&self, expected: Vec<DictKey>) -> StrResult<()> {
        let mut iter = self.iter().peekable();
        if iter.peek().is_none() {
            return Ok(());
        }
        let unexpected: Vec<DictKey> = iter.map(|kv| kv.0.clone()).collect();

        Err(Self::unexpected_keys(unexpected, Some(expected)))
    }

    // Return an "unexpected key" error string.
    pub fn unexpected_keys(
        unexpected: Vec<DictKey>,
        hint_expected: Option<Vec<DictKey>>,
    ) -> EcoString {
        let format_as_list = |arr: &Vec<DictKey>| {
            repr::separated_list(
                &arr.iter().map(|k| eco_format!("\"{k}\"")).collect::<Vec<_>>(),
                "and",
            )
        };

        let mut msg = String::from(match unexpected.len() {
            1 => "unexpected key ",
            _ => "unexpected keys ",
        });

        msg.push_str(&format_as_list(&unexpected));

        if let Some(expected) = hint_expected {
            msg.push_str(", valid keys are ");
            msg.push_str(&format_as_list(&expected));
        }

        msg.into()
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
        key: DictKey,
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

    /// Inserts a new pair into the dictionary. If the dictionary already
    /// contains this key, the value is updated.
    #[func]
    pub fn insert(
        &mut self,
        /// The key of the pair that should be inserted.
        key: DictKey,
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
        key: DictKey,
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
        self.0.keys().map(|k| k.clone().into_value()).collect()
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
            .map(|(key, value)| eco_format!("{key}: {}", value.repr()))
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

impl Serialize for DictKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            DictKey::Str(s) => s.serialize(serializer),
            DictKey::Int(i) => i.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for DictKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Str::deserialize(deserializer)?.into())
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
        Ok(IndexMap::<DictKey, Value>::deserialize(deserializer)?.into())
    }
}

impl Extend<(DictKey, Value)> for Dict {
    fn extend<T: IntoIterator<Item = (DictKey, Value)>>(&mut self, iter: T) {
        Arc::make_mut(&mut self.0).extend(iter);
    }
}

impl FromIterator<(DictKey, Value)> for Dict {
    fn from_iter<T: IntoIterator<Item = (DictKey, Value)>>(iter: T) -> Self {
        Self(Arc::new(iter.into_iter().collect()))
    }
}

impl IntoIterator for Dict {
    type Item = (DictKey, Value);
    type IntoIter = indexmap::map::IntoIter<DictKey, Value>;

    fn into_iter(self) -> Self::IntoIter {
        Arc::take(self.0).into_iter()
    }
}

impl<'a> IntoIterator for &'a Dict {
    type Item = (&'a DictKey, &'a Value);
    type IntoIter = indexmap::map::Iter<'a, DictKey, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl From<IndexMap<DictKey, Value>> for Dict {
    fn from(map: IndexMap<DictKey, Value>) -> Self {
        Self(Arc::new(map))
    }
}

/// The missing key access error message.
#[cold]
fn missing_key(key: &DictKey) -> EcoString {
    eco_format!("dictionary does not contain key {}", key.repr())
}

/// The missing key access error message when no default was fiven.
#[cold]
fn missing_key_no_default(key: &DictKey) -> EcoString {
    eco_format!(
        "dictionary does not contain key {} \
         and no default value was specified",
        key.repr()
    )
}
