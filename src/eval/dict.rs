use std::collections::BTreeMap;
use std::fmt::{self, Debug, Display, Formatter, Write};
use std::iter::FromIterator;
use std::ops::{Add, AddAssign};
use std::rc::Rc;

use super::{Str, Value};
use crate::diag::StrResult;

/// Create a new [`Dict`] from key-value pairs.
#[allow(unused_macros)]
macro_rules! dict {
    ($($key:expr => $value:expr),* $(,)?) => {{
        #[allow(unused_mut)]
        let mut map = std::collections::BTreeMap::new();
        $(map.insert($crate::eval::Str::from($key), $crate::eval::Value::from($value));)*
        $crate::eval::Dict::from_map(map)
    }};
}

/// A dictionary from strings to values with clone-on-write value semantics.
#[derive(Default, Clone, PartialEq)]
pub struct Dict {
    map: Rc<BTreeMap<Str, Value>>,
}

impl Dict {
    /// Create a new, empty dictionary.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new dictionary from a mapping of strings to values.
    pub fn from_map(map: BTreeMap<Str, Value>) -> Self {
        Self { map: Rc::new(map) }
    }

    /// Whether the dictionary is empty.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// The number of pairs in the dictionary.
    pub fn len(&self) -> i64 {
        self.map.len() as i64
    }

    /// Borrow the value the given `key` maps to.
    pub fn get(&self, key: Str) -> StrResult<&Value> {
        self.map.get(&key).ok_or_else(|| missing_key(&key))
    }

    /// Mutably borrow the value the given `key` maps to.
    ///
    /// This inserts the key with [`None`](Value::None) as the value if not
    /// present so far.
    pub fn get_mut(&mut self, key: Str) -> &mut Value {
        Rc::make_mut(&mut self.map).entry(key.into()).or_default()
    }

    /// Insert a mapping from the given `key` to the given `value`.
    pub fn insert(&mut self, key: Str, value: Value) {
        Rc::make_mut(&mut self.map).insert(key.into(), value);
    }

    /// Clear the dictionary.
    pub fn clear(&mut self) {
        if Rc::strong_count(&mut self.map) == 1 {
            Rc::make_mut(&mut self.map).clear();
        } else {
            *self = Self::new();
        }
    }

    /// Iterate over pairs of references to the contained keys and values.
    pub fn iter(&self) -> std::collections::btree_map::Iter<Str, Value> {
        self.map.iter()
    }
}

/// The missing key access error message.
#[cold]
fn missing_key(key: &Str) -> String {
    format!("dictionary does not contain key: {}", key)
}

impl Display for Dict {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_char('(')?;
        if self.is_empty() {
            f.write_char(':')?;
        }
        for (i, (key, value)) in self.iter().enumerate() {
            f.write_str(key)?;
            f.write_str(": ")?;
            Display::fmt(value, f)?;
            if i + 1 < self.map.len() {
                f.write_str(", ")?;
            }
        }
        f.write_char(')')
    }
}

impl Debug for Dict {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_map().entries(self.map.iter()).finish()
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
        match Rc::try_unwrap(rhs.map) {
            Ok(map) => self.extend(map),
            Err(rc) => self.extend(rc.iter().map(|(k, v)| (k.clone(), v.clone()))),
        }
    }
}

impl FromIterator<(Str, Value)> for Dict {
    fn from_iter<T: IntoIterator<Item = (Str, Value)>>(iter: T) -> Self {
        Dict { map: Rc::new(iter.into_iter().collect()) }
    }
}

impl Extend<(Str, Value)> for Dict {
    fn extend<T: IntoIterator<Item = (Str, Value)>>(&mut self, iter: T) {
        Rc::make_mut(&mut self.map).extend(iter);
    }
}

impl IntoIterator for Dict {
    type Item = (Str, Value);
    type IntoIter = std::collections::btree_map::IntoIter<Str, Value>;

    fn into_iter(self) -> Self::IntoIter {
        match Rc::try_unwrap(self.map) {
            Ok(map) => map.into_iter(),
            Err(rc) => (*rc).clone().into_iter(),
        }
    }
}

impl<'a> IntoIterator for &'a Dict {
    type Item = (&'a Str, &'a Value);
    type IntoIter = std::collections::btree_map::Iter<'a, Str, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
