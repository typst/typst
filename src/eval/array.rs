use std::fmt::{self, Debug, Formatter};
use std::iter::FromIterator;
use std::ops::{Add, AddAssign};
use std::rc::Rc;

use super::Value;

/// Create a new [`Array`] from values.
#[macro_export]
macro_rules! array {
    ($value:expr; $count:expr) => {
        $crate::eval::Array::from_vec(vec![$crate::eval::Value::from($value); $count])
    };

    ($($value:expr),* $(,)?) => {
        $crate::eval::Array::from_vec(vec![$($crate::eval::Value::from($value)),*])
    };
}

/// A variably-typed array with clone-on-write value semantics.
#[derive(Clone, PartialEq)]
pub struct Array {
    vec: Rc<Vec<Value>>,
}

impl Array {
    /// Create a new, empty array.
    pub fn new() -> Self {
        Self { vec: Rc::new(vec![]) }
    }

    /// Create a new array from a vector of values.
    pub fn from_vec(vec: Vec<Value>) -> Self {
        Self { vec: Rc::new(vec) }
    }

    /// Create a new, empty array with the given `capacity`.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            vec: Rc::new(Vec::with_capacity(capacity)),
        }
    }

    /// Whether the array is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The length of the array.
    pub fn len(&self) -> usize {
        self.vec.len()
    }

    /// Borrow the value at the given index.
    pub fn get(&self, index: usize) -> Option<&Value> {
        self.vec.get(index)
    }

    /// Mutably borrow the value at the given index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Value> {
        Rc::make_mut(&mut self.vec).get_mut(index)
    }

    /// Set the value at the given index.
    ///
    /// This panics the `index` is out of range.
    pub fn set(&mut self, index: usize, value: Value) {
        Rc::make_mut(&mut self.vec)[index] = value;
    }

    /// Push a value to the end of the array.
    pub fn push(&mut self, value: Value) {
        Rc::make_mut(&mut self.vec).push(value);
    }

    /// Clear the array.
    pub fn clear(&mut self) {
        if Rc::strong_count(&mut self.vec) == 1 {
            Rc::make_mut(&mut self.vec).clear();
        } else {
            *self = Self::new();
        }
    }

    /// Repeat this array `n` times.
    pub fn repeat(&self, n: usize) -> Self {
        let len = self.len().checked_mul(n).expect("capacity overflow");
        self.iter().cloned().cycle().take(len).collect()
    }

    /// Iterate over references to the contained values.
    pub fn iter(&self) -> std::slice::Iter<Value> {
        self.vec.iter()
    }
}

impl Default for Array {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for Array {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_list().entries(self.vec.iter()).finish()
    }
}

impl Add for Array {
    type Output = Self;

    fn add(mut self, rhs: Array) -> Self::Output {
        self += rhs;
        self
    }
}

impl AddAssign for Array {
    fn add_assign(&mut self, rhs: Array) {
        match Rc::try_unwrap(rhs.vec) {
            Ok(vec) => self.extend(vec),
            Err(rc) => self.extend(rc.iter().cloned()),
        }
    }
}

impl FromIterator<Value> for Array {
    fn from_iter<T: IntoIterator<Item = Value>>(iter: T) -> Self {
        Array { vec: Rc::new(iter.into_iter().collect()) }
    }
}

impl Extend<Value> for Array {
    fn extend<T: IntoIterator<Item = Value>>(&mut self, iter: T) {
        Rc::make_mut(&mut self.vec).extend(iter);
    }
}

impl IntoIterator for Array {
    type Item = Value;
    type IntoIter = std::vec::IntoIter<Value>;

    fn into_iter(self) -> Self::IntoIter {
        match Rc::try_unwrap(self.vec) {
            Ok(vec) => vec.into_iter(),
            Err(rc) => (*rc).clone().into_iter(),
        }
    }
}

impl<'a> IntoIterator for &'a Array {
    type Item = &'a Value;
    type IntoIter = std::slice::Iter<'a, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
