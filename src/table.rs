//! A table data structure.

use std::collections::BTreeMap;
use std::fmt::{self, Debug, Formatter};
use std::ops::Index;

/// A table is a key-value map that can also model array-like structures.
///
/// An array-like table assigns value to successive indices from `0..n`. The
/// table type offers special support for this pattern through the `push`
/// method.
///
/// The keys of a table may be strings or integers (`u64`). The table is generic
/// over the value type.
#[derive(Clone)]
pub struct Table<V> {
    nums: BTreeMap<u64, V>,
    strs: BTreeMap<String, V>,
    lowest_free: u64,
}

impl<V> Table<V> {
    /// Create a new empty table.
    pub fn new() -> Self {
        Self {
            nums: BTreeMap::new(),
            strs: BTreeMap::new(),
            lowest_free: 0,
        }
    }

    /// The total number of entries in the table.
    pub fn len(&self) -> usize {
        self.nums.len() + self.strs.len()
    }

    /// Whether the table contains no entries.
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
        K: Into<BorrowedKey<'a>>,
    {
        match key.into() {
            BorrowedKey::Num(num) => self.nums.get(&num),
            BorrowedKey::Str(string) => self.strs.get(string),
        }
    }

    /// Borrow the value with the given key mutably.
    pub fn get_mut<'a, K>(&mut self, key: K) -> Option<&mut V>
    where
        K: Into<BorrowedKey<'a>>,
    {
        match key.into() {
            BorrowedKey::Num(num) => self.nums.get_mut(&num),
            BorrowedKey::Str(string) => self.strs.get_mut(string),
        }
    }

    /// Insert a value into the table.
    pub fn insert<K>(&mut self, key: K, value: V)
    where
        K: Into<OwnedKey>,
    {
        match key.into() {
            OwnedKey::Num(num) => {
                self.nums.insert(num, value);
                if self.lowest_free == num {
                    self.lowest_free += 1;
                }
            }
            OwnedKey::Str(string) => {
                self.strs.insert(string, value);
            }
        }
    }

    /// Remove the value with the given key from the table.
    pub fn remove<'a, K>(&mut self, key: K) -> Option<V>
    where
        K: Into<BorrowedKey<'a>>,
    {
        match key.into() {
            BorrowedKey::Num(num) => {
                self.lowest_free = self.lowest_free.min(num);
                self.nums.remove(&num)
            }
            BorrowedKey::Str(string) => self.strs.remove(string),
        }
    }

    /// Append a value to the table.
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

    /// Iterate over the number key-value pairs.
    pub fn nums(&self) -> std::collections::btree_map::Iter<u64, V> {
        self.nums.iter()
    }

    /// Iterate over the string key-value pairs.
    pub fn strs(&self) -> std::collections::btree_map::Iter<String, V> {
        self.strs.iter()
    }

    /// Iterate over all values in the table.
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.nums().map(|(_, v)| v).chain(self.strs().map(|(_, v)| v))
    }

    /// Iterate over the number key-value pairs.
    pub fn into_nums(self) -> std::collections::btree_map::IntoIter<u64, V> {
        self.nums.into_iter()
    }

    /// Iterate over the string key-value pairs.
    pub fn into_strs(self) -> std::collections::btree_map::IntoIter<String, V> {
        self.strs.into_iter()
    }

    /// Move into an owned iterator over all values in the table.
    pub fn into_values(self) -> impl Iterator<Item = V> {
        self.nums.into_iter().map(|(_, v)| v)
            .chain(self.strs.into_iter().map(|(_, v)| v))
    }
}

impl<'a, K, V> Index<K> for Table<V>
where
    K: Into<BorrowedKey<'a>>,
{
    type Output = V;

    fn index(&self, index: K) -> &Self::Output {
        self.get(index).expect("key not in table")
    }
}

impl<V> Default for Table<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Eq> Eq for Table<V> {}

impl<V: PartialEq> PartialEq for Table<V> {
    fn eq(&self, other: &Self) -> bool {
        self.nums().eq(other.nums()) && self.strs().eq(other.strs())
    }
}

impl<V: Debug> Debug for Table<V> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.is_empty() {
            return f.write_str("()");
        }

        let mut builder = f.debug_tuple("");

        struct Entry<'a>(&'a dyn Debug, &'a dyn Debug);
        impl<'a> Debug for Entry<'a> {
            fn fmt(&self, f: &mut Formatter) -> fmt::Result {
                self.0.fmt(f)?;
                if f.alternate() {
                    f.write_str(" = ")?;
                } else {
                    f.write_str("=")?;
                }
                self.1.fmt(f)
            }
        }

        for (key, value) in self.nums() {
            builder.field(&Entry(&key, &value));
        }

        for (key, value) in self.strs() {
            builder.field(&Entry(&key, &value));
        }

        builder.finish()
    }
}

/// The owned variant of a table key.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum OwnedKey {
    Num(u64),
    Str(String),
}

impl From<u64> for OwnedKey {
    fn from(num: u64) -> Self {
        Self::Num(num)
    }
}

impl From<String> for OwnedKey {
    fn from(string: String) -> Self {
        Self::Str(string)
    }
}

impl From<&str> for OwnedKey {
    fn from(string: &str) -> Self {
        Self::Str(string.to_string())
    }
}

/// The borrowed variant of a table key.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum BorrowedKey<'a> {
    Num(u64),
    Str(&'a str),
}

impl From<u64> for BorrowedKey<'static> {
    fn from(num: u64) -> Self {
        Self::Num(num)
    }
}

impl<'a> From<&'a String> for BorrowedKey<'a> {
    fn from(string: &'a String) -> Self {
        Self::Str(&string)
    }
}

impl<'a> From<&'a str> for BorrowedKey<'a> {
    fn from(string: &'a str) -> Self {
        Self::Str(string)
    }
}

#[cfg(test)]
mod tests {
    use super::Table;

    #[test]
    fn test_table_different_key_types_dont_interfere() {
        let mut table = Table::new();
        table.insert(10, "hello");
        table.insert("twenty", "there");
        assert_eq!(table.len(), 2);
        assert_eq!(table[10], "hello");
        assert_eq!(table["twenty"], "there");
    }

    #[test]
    fn test_table_push_skips_already_inserted_keys() {
        let mut table = Table::new();
        table.insert(2, "2");
        table.push("0");
        table.insert(3, "3");
        table.push("1");
        table.push("4");
        assert_eq!(table.len(), 5);
        assert_eq!(table[0], "0");
        assert_eq!(table[1], "1");
        assert_eq!(table[2], "2");
        assert_eq!(table[3], "3");
        assert_eq!(table[4], "4");
    }

    #[test]
    fn test_table_push_remove_push_reuses_index() {
        let mut table = Table::new();
        table.push("0");
        table.push("1");
        table.push("2");
        table.remove(1);
        table.push("a");
        table.push("3");
        assert_eq!(table.len(), 4);
        assert_eq!(table[0], "0");
        assert_eq!(table[1], "a");
        assert_eq!(table[2], "2");
        assert_eq!(table[3], "3");
    }

    #[test]
    fn test_table_first_and_last_are_correct() {
        let mut table = Table::new();
        assert_eq!(table.first(), None);
        assert_eq!(table.last(), None);
        table.insert(4, "hi");
        table.insert("string", "hi");
        assert_eq!(table.first(), Some((4, &"hi")));
        assert_eq!(table.last(), Some((4, &"hi")));
        table.insert(2, "bye");
        assert_eq!(table.first(), Some((2, &"bye")));
        assert_eq!(table.last(), Some((4, &"hi")));
    }

    #[test]
    fn test_table_format_debug() {
        let mut table = Table::new();
        assert_eq!(format!("{:?}", table), r#"()"#);
        assert_eq!(format!("{:#?}", table), r#"()"#);

        table.insert(10, "hello");
        table.insert("twenty", "there");
        table.insert("sp ace", "quotes");
        assert_eq!(
            format!("{:?}", table),
            r#"(10="hello", "sp ace"="quotes", "twenty"="there")"#,
        );
        assert_eq!(format!("{:#?}", table).lines().collect::<Vec<_>>(), [
            "(",
            r#"    10 = "hello","#,
            r#"    "sp ace" = "quotes","#,
            r#"    "twenty" = "there","#,
            ")",
        ]);
    }
}
