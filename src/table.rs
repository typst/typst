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
#[derive(Default, Clone, PartialEq)]
pub struct Table<V> {
    nums: BTreeMap<u64, V>,
    strings: BTreeMap<String, V>,
    lowest_free: u64,
}

impl<V> Table<V> {
    /// Create a new empty table.
    pub fn new() -> Self {
        Self {
            nums: BTreeMap::new(),
            strings: BTreeMap::new(),
            lowest_free: 0,
        }
    }

    /// The total number of entries in the table.
    pub fn len(&self) -> usize {
        self.nums.len() + self.strings.len()
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
            BorrowedKey::Number(num) => self.nums.get(&num),
            BorrowedKey::Str(string) => self.strings.get(string),
        }
    }

    /// Borrow the value with the given key mutably.
    pub fn get_mut<'a, K>(&mut self, key: K) -> Option<&mut V>
    where
        K: Into<BorrowedKey<'a>>,
    {
        match key.into() {
            BorrowedKey::Number(num) => self.nums.get_mut(&num),
            BorrowedKey::Str(string) => self.strings.get_mut(string),
        }
    }

    /// Insert a value into the table.
    pub fn insert<K>(&mut self, key: K, value: V)
    where
        K: Into<OwnedKey>,
    {
        match key.into() {
            OwnedKey::Number(num) => {
                self.nums.insert(num, value);
                if self.lowest_free == num {
                    self.lowest_free += 1;
                }
            }
            OwnedKey::Str(string) => {
                self.strings.insert(string, value);
            }
        }
    }

    /// Remove the value with the given key from the table.
    pub fn remove<'a, K>(&mut self, key: K) -> Option<V>
    where
        K: Into<BorrowedKey<'a>>,
    {
        match key.into() {
            BorrowedKey::Number(num) => {
                self.lowest_free = self.lowest_free.min(num);
                self.nums.remove(&num)
            }
            BorrowedKey::Str(string) => self.strings.remove(string),
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

impl<V: Debug> Debug for Table<V> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("(")?;
        if f.alternate() && (!self.nums.is_empty() || !self.strings.is_empty()) {
            f.write_str("\n")?;
        }

        let len = self.len();
        let nums = self.nums.iter().map(|(k, v)| (k as &dyn Debug, v));
        let strings = self.strings.iter().map(|(k, v)| (k as &dyn Debug, v));
        let pairs = nums.chain(strings);

        for (i, (key, value)) in pairs.enumerate() {
            if f.alternate() {
                f.write_str("    ")?;
            }
            key.fmt(f)?;
            if f.alternate() {
                f.write_str(" = ")?;
            } else {
                f.write_str("=")?;
            }
            value.fmt(f)?;
            if f.alternate() {
                f.write_str(",\n")?;
            } else if i + 1 < len {
                f.write_str(", ")?;
            }
        }

        f.write_str(")")
    }
}

/// The owned variant of a table key.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum OwnedKey {
    Number(u64),
    Str(String),
}

impl From<u64> for OwnedKey {
    fn from(num: u64) -> Self {
        Self::Number(num)
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
    Number(u64),
    Str(&'a str),
}

impl From<u64> for BorrowedKey<'static> {
    fn from(num: u64) -> Self {
        Self::Number(num)
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
