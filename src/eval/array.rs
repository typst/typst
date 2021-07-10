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

    /// Extend the array with the values from another array.
    pub fn extend(&mut self, other: &Array) {
        Rc::make_mut(&mut self.vec).extend(other.into_iter())
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
        self.into_iter().cycle().take(len).collect()
    }

    /// Iterate over references to the contained values.
    pub fn iter(&self) -> std::slice::Iter<Value> {
        self.vec.iter()
    }

    /// Iterate over the contained values.
    pub fn into_iter(&self) -> impl Iterator<Item = Value> + Clone + '_ {
        // TODO: Actually consume the vector if the ref-count is 1?
        self.iter().cloned()
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

impl FromIterator<Value> for Array {
    fn from_iter<T: IntoIterator<Item = Value>>(iter: T) -> Self {
        Array { vec: Rc::new(iter.into_iter().collect()) }
    }
}

impl<'a> IntoIterator for &'a Array {
    type Item = &'a Value;
    type IntoIter = std::slice::Iter<'a, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl Add<&Array> for Array {
    type Output = Self;

    fn add(mut self, rhs: &Array) -> Self::Output {
        self.extend(rhs);
        self
    }
}

impl AddAssign<&Array> for Array {
    fn add_assign(&mut self, rhs: &Array) {
        self.extend(rhs);
    }
}
