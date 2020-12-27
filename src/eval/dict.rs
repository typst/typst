//! A key-value map that can also model array-like structures.

use std::collections::BTreeMap;
use std::fmt::{self, Debug, Display, Formatter};
use std::iter::{Extend, FromIterator};
use std::ops::Index;

use crate::syntax::{Span, Spanned};

/// A dictionary data structure, which maps from integers and strings to a
/// generic value type.
///
/// The dictionary can be used to model arrays by assigning values to successive
/// indices from `0..n`. The `push` method offers special support for this
/// pattern.
#[derive(Clone)]
pub struct Dict<V> {
    nums: BTreeMap<u64, V>,
    strs: BTreeMap<String, V>,
    lowest_free: u64,
}

impl<V> Dict<V> {
    /// Create a new empty dictionary.
    pub fn new() -> Self {
        Self {
            nums: BTreeMap::new(),
            strs: BTreeMap::new(),
            lowest_free: 0,
        }
    }

    /// The total number of entries in the dictionary.
    pub fn len(&self) -> usize {
        self.nums.len() + self.strs.len()
    }

    /// Whether the dictionary contains no entries.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The first number key-value pair (with lowest number).
    pub fn first(&self) -> Option<(u64, &V)> {
        self.nums.iter().next().map(|(&k, v)| (k, v))
    }

    /// The last number key-value pair (with highest number).
    pub fn last(&self) -> Option<(u64, &V)> {
        self.nums.iter().next_back().map(|(&k, v)| (k, v))
    }

    /// Get a reference to the value with the given key.
    pub fn get<'a, K>(&self, key: K) -> Option<&V>
    where
        K: Into<RefKey<'a>>,
    {
        match key.into() {
            RefKey::Num(num) => self.nums.get(&num),
            RefKey::Str(string) => self.strs.get(string),
        }
    }

    /// Borrow the value with the given key mutably.
    pub fn get_mut<'a, K>(&mut self, key: K) -> Option<&mut V>
    where
        K: Into<RefKey<'a>>,
    {
        match key.into() {
            RefKey::Num(num) => self.nums.get_mut(&num),
            RefKey::Str(string) => self.strs.get_mut(string),
        }
    }

    /// Insert a value into the dictionary.
    pub fn insert<K>(&mut self, key: K, value: V)
    where
        K: Into<DictKey>,
    {
        match key.into() {
            DictKey::Num(num) => {
                self.nums.insert(num, value);
                if self.lowest_free == num {
                    self.lowest_free += 1;
                }
            }
            DictKey::Str(string) => {
                self.strs.insert(string, value);
            }
        }
    }

    /// Remove the value with the given key from the dictionary.
    pub fn remove<'a, K>(&mut self, key: K) -> Option<V>
    where
        K: Into<RefKey<'a>>,
    {
        match key.into() {
            RefKey::Num(num) => {
                self.lowest_free = self.lowest_free.min(num);
                self.nums.remove(&num)
            }
            RefKey::Str(string) => self.strs.remove(string),
        }
    }

    /// Append a value to the dictionary.
    ///
    /// This will associate the `value` with the lowest free number key (zero if
    /// there is no number key so far).
    pub fn push(&mut self, value: V) {
        while self.nums.contains_key(&self.lowest_free) {
            self.lowest_free += 1;
        }
        self.nums.insert(self.lowest_free, value);
        self.lowest_free += 1;
    }
}

impl<'a, K, V> Index<K> for Dict<V>
where
    K: Into<RefKey<'a>>,
{
    type Output = V;

    fn index(&self, index: K) -> &Self::Output {
        self.get(index).expect("key not in dict")
    }
}

impl<V: Eq> Eq for Dict<V> {}

impl<V: PartialEq> PartialEq for Dict<V> {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}

impl<V> Default for Dict<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Debug> Debug for Dict<V> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.is_empty() {
            return f.write_str("()");
        }

        let mut builder = f.debug_tuple("");

        struct Entry<'a>(bool, &'a dyn Display, &'a dyn Debug);
        impl<'a> Debug for Entry<'a> {
            fn fmt(&self, f: &mut Formatter) -> fmt::Result {
                if self.0 {
                    f.write_str("\"")?;
                }
                self.1.fmt(f)?;
                if self.0 {
                    f.write_str("\"")?;
                }

                f.write_str(": ")?;

                self.2.fmt(f)
            }
        }

        for (key, value) in self.nums() {
            builder.field(&Entry(false, &key, &value));
        }

        for (key, value) in self.strs() {
            builder.field(&Entry(key.contains(' '), &key, &value));
        }

        builder.finish()
    }
}

/// Iteration.
impl<V> Dict<V> {
    /// Iterator over all borrowed keys and values.
    pub fn iter(&self) -> impl Iterator<Item = (RefKey, &V)> {
        self.into_iter()
    }

    /// Iterator over all borrowed keys and values.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (RefKey, &mut V)> {
        self.into_iter()
    }

    /// Iterate over all values in the dictionary.
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.nums().map(|(_, v)| v).chain(self.strs().map(|(_, v)| v))
    }

    /// Iterate over all values in the dictionary.
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.nums
            .iter_mut()
            .map(|(_, v)| v)
            .chain(self.strs.iter_mut().map(|(_, v)| v))
    }

    /// Move into an owned iterator over all values in the dictionary.
    pub fn into_values(self) -> impl Iterator<Item = V> {
        self.nums
            .into_iter()
            .map(|(_, v)| v)
            .chain(self.strs.into_iter().map(|(_, v)| v))
    }

    /// Iterate over the number key-value pairs.
    pub fn nums(&self) -> std::collections::btree_map::Iter<u64, V> {
        self.nums.iter()
    }

    /// Iterate mutably over the number key-value pairs.
    pub fn nums_mut(&mut self) -> std::collections::btree_map::IterMut<u64, V> {
        self.nums.iter_mut()
    }

    /// Iterate over the number key-value pairs.
    pub fn into_nums(self) -> std::collections::btree_map::IntoIter<u64, V> {
        self.nums.into_iter()
    }

    /// Iterate over the string key-value pairs.
    pub fn strs(&self) -> std::collections::btree_map::Iter<String, V> {
        self.strs.iter()
    }

    /// Iterate mutably over the string key-value pairs.
    pub fn strs_mut(&mut self) -> std::collections::btree_map::IterMut<String, V> {
        self.strs.iter_mut()
    }

    /// Iterate over the string key-value pairs.
    pub fn into_strs(self) -> std::collections::btree_map::IntoIter<String, V> {
        self.strs.into_iter()
    }
}

impl<V> Extend<(DictKey, V)> for Dict<V> {
    fn extend<T: IntoIterator<Item = (DictKey, V)>>(&mut self, iter: T) {
        for (key, value) in iter.into_iter() {
            self.insert(key, value);
        }
    }
}

impl<V> FromIterator<(DictKey, V)> for Dict<V> {
    fn from_iter<T: IntoIterator<Item = (DictKey, V)>>(iter: T) -> Self {
        let mut v = Self::new();
        v.extend(iter);
        v
    }
}

impl<V> IntoIterator for Dict<V> {
    type Item = (DictKey, V);
    type IntoIter = std::iter::Chain<
        std::iter::Map<
            std::collections::btree_map::IntoIter<u64, V>,
            fn((u64, V)) -> (DictKey, V),
        >,
        std::iter::Map<
            std::collections::btree_map::IntoIter<String, V>,
            fn((String, V)) -> (DictKey, V),
        >,
    >;

    fn into_iter(self) -> Self::IntoIter {
        let nums = self.nums.into_iter().map((|(k, v)| (DictKey::Num(k), v)) as _);
        let strs = self.strs.into_iter().map((|(k, v)| (DictKey::Str(k), v)) as _);
        nums.chain(strs)
    }
}

impl<'a, V> IntoIterator for &'a Dict<V> {
    type Item = (RefKey<'a>, &'a V);
    type IntoIter = std::iter::Chain<
        std::iter::Map<
            std::collections::btree_map::Iter<'a, u64, V>,
            fn((&'a u64, &'a V)) -> (RefKey<'a>, &'a V),
        >,
        std::iter::Map<
            std::collections::btree_map::Iter<'a, String, V>,
            fn((&'a String, &'a V)) -> (RefKey<'a>, &'a V),
        >,
    >;

    fn into_iter(self) -> Self::IntoIter {
        let nums = self.nums().map((|(k, v): (&u64, _)| (RefKey::Num(*k), v)) as _);
        let strs = self.strs().map((|(k, v): (&'a String, _)| (RefKey::Str(k), v)) as _);
        nums.chain(strs)
    }
}

impl<'a, V> IntoIterator for &'a mut Dict<V> {
    type Item = (RefKey<'a>, &'a mut V);
    type IntoIter = std::iter::Chain<
        std::iter::Map<
            std::collections::btree_map::IterMut<'a, u64, V>,
            fn((&'a u64, &'a mut V)) -> (RefKey<'a>, &'a mut V),
        >,
        std::iter::Map<
            std::collections::btree_map::IterMut<'a, String, V>,
            fn((&'a String, &'a mut V)) -> (RefKey<'a>, &'a mut V),
        >,
    >;

    fn into_iter(self) -> Self::IntoIter {
        let nums = self
            .nums
            .iter_mut()
            .map((|(k, v): (&u64, _)| (RefKey::Num(*k), v)) as _);
        let strs = self
            .strs
            .iter_mut()
            .map((|(k, v): (&'a String, _)| (RefKey::Str(k), v)) as _);
        nums.chain(strs)
    }
}

/// The owned variant of a dictionary key.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum DictKey {
    Num(u64),
    Str(String),
}

impl From<&Self> for DictKey {
    fn from(key: &Self) -> Self {
        key.clone()
    }
}

impl From<RefKey<'_>> for DictKey {
    fn from(key: RefKey<'_>) -> Self {
        match key {
            RefKey::Num(num) => Self::Num(num),
            RefKey::Str(string) => Self::Str(string.to_string()),
        }
    }
}

impl From<u64> for DictKey {
    fn from(num: u64) -> Self {
        Self::Num(num)
    }
}

impl From<String> for DictKey {
    fn from(string: String) -> Self {
        Self::Str(string)
    }
}

impl From<&'static str> for DictKey {
    fn from(string: &'static str) -> Self {
        Self::Str(string.to_string())
    }
}

/// The borrowed variant of a dictionary key.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum RefKey<'a> {
    Num(u64),
    Str(&'a str),
}

impl From<u64> for RefKey<'static> {
    fn from(num: u64) -> Self {
        Self::Num(num)
    }
}

impl<'a> From<&'a String> for RefKey<'a> {
    fn from(string: &'a String) -> Self {
        Self::Str(&string)
    }
}

impl<'a> From<&'a str> for RefKey<'a> {
    fn from(string: &'a str) -> Self {
        Self::Str(string)
    }
}

/// A dictionary entry which combines key span and value.
///
/// This exists because a key in a directory can't track its span by itself.
#[derive(Clone, PartialEq)]
pub struct SpannedEntry<V> {
    pub key_span: Span,
    pub value: Spanned<V>,
}

impl<V> SpannedEntry<V> {
    /// Create a new entry.
    pub fn new(key: Span, val: Spanned<V>) -> Self {
        Self { key_span: key, value: val }
    }

    /// Create an entry with the same span for key and value.
    pub fn value(val: Spanned<V>) -> Self {
        Self { key_span: val.span, value: val }
    }

    /// Convert from `&SpannedEntry<T>` to `SpannedEntry<&T>`
    pub fn as_ref(&self) -> SpannedEntry<&V> {
        SpannedEntry {
            key_span: self.key_span,
            value: self.value.as_ref(),
        }
    }

    /// Map the entry to a different value type.
    pub fn map<U>(self, f: impl FnOnce(V) -> U) -> SpannedEntry<U> {
        SpannedEntry {
            key_span: self.key_span,
            value: self.value.map(f),
        }
    }
}

impl<V: Debug> Debug for SpannedEntry<V> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if f.alternate() {
            f.write_str("key")?;
            self.key_span.fmt(f)?;
            f.write_str(" ")?;
        }
        self.value.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::Dict;

    #[test]
    fn test_dict_different_key_types_dont_interfere() {
        let mut dict = Dict::new();
        dict.insert(10, "hello");
        dict.insert("twenty", "there");
        assert_eq!(dict.len(), 2);
        assert_eq!(dict[10], "hello");
        assert_eq!(dict["twenty"], "there");
    }

    #[test]
    fn test_dict_push_skips_already_inserted_keys() {
        let mut dict = Dict::new();
        dict.insert(2, "2");
        dict.push("0");
        dict.insert(3, "3");
        dict.push("1");
        dict.push("4");
        assert_eq!(dict.len(), 5);
        assert_eq!(dict[0], "0");
        assert_eq!(dict[1], "1");
        assert_eq!(dict[2], "2");
        assert_eq!(dict[3], "3");
        assert_eq!(dict[4], "4");
    }

    #[test]
    fn test_dict_push_remove_push_reuses_index() {
        let mut dict = Dict::new();
        dict.push("0");
        dict.push("1");
        dict.push("2");
        dict.remove(1);
        dict.push("a");
        dict.push("3");
        assert_eq!(dict.len(), 4);
        assert_eq!(dict[0], "0");
        assert_eq!(dict[1], "a");
        assert_eq!(dict[2], "2");
        assert_eq!(dict[3], "3");
    }

    #[test]
    fn test_dict_first_and_last_are_correct() {
        let mut dict = Dict::new();
        assert_eq!(dict.first(), None);
        assert_eq!(dict.last(), None);
        dict.insert(4, "hi");
        dict.insert("string", "hi");
        assert_eq!(dict.first(), Some((4, &"hi")));
        assert_eq!(dict.last(), Some((4, &"hi")));
        dict.insert(2, "bye");
        assert_eq!(dict.first(), Some((2, &"bye")));
        assert_eq!(dict.last(), Some((4, &"hi")));
    }

    #[test]
    fn test_dict_format_debug() {
        let mut dict = Dict::new();
        assert_eq!(format!("{:?}", dict), "()");
        assert_eq!(format!("{:#?}", dict), "()");

        dict.insert(10, "hello");
        dict.insert("twenty", "there");
        dict.insert("sp ace", "quotes");
        assert_eq!(
            format!("{:?}", dict),
            r#"(10: "hello", "sp ace": "quotes", twenty: "there")"#,
        );
        assert_eq!(format!("{:#?}", dict).lines().collect::<Vec<_>>(), [
            "(",
            r#"    10: "hello","#,
            r#"    "sp ace": "quotes","#,
            r#"    twenty: "there","#,
            ")",
        ]);
    }
}
