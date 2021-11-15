use std::cmp::Ordering;
use std::convert::TryFrom;
use std::fmt::{self, Debug, Formatter, Write};
use std::iter::FromIterator;
use std::ops::{Add, AddAssign};
use std::rc::Rc;

use super::Value;
use crate::diag::StrResult;
use crate::util::RcExt;

/// Create a new [`Array`] from values.
#[allow(unused_macros)]
macro_rules! array {
    ($value:expr; $count:expr) => {
        $crate::eval::Array::from_vec(vec![$value.into(); $count])
    };

    ($($value:expr),* $(,)?) => {
        $crate::eval::Array::from_vec(vec![$($value.into()),*])
    };
}

/// An array of values with clone-on-write value semantics.
#[derive(Default, Clone, PartialEq)]
pub struct Array(Rc<Vec<Value>>);

impl Array {
    /// Create a new, empty array.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new array from a vector of values.
    pub fn from_vec(vec: Vec<Value>) -> Self {
        Self(Rc::new(vec))
    }

    /// Whether the array is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The length of the array.
    pub fn len(&self) -> i64 {
        self.0.len() as i64
    }

    /// Borrow the value at the given index.
    pub fn get(&self, index: i64) -> StrResult<&Value> {
        usize::try_from(index)
            .ok()
            .and_then(|i| self.0.get(i))
            .ok_or_else(|| out_of_bounds(index, self.len()))
    }

    /// Mutably borrow the value at the given index.
    pub fn get_mut(&mut self, index: i64) -> StrResult<&mut Value> {
        let len = self.len();
        usize::try_from(index)
            .ok()
            .and_then(move |i| Rc::make_mut(&mut self.0).get_mut(i))
            .ok_or_else(|| out_of_bounds(index, len))
    }

    /// Push a value to the end of the array.
    pub fn push(&mut self, value: Value) {
        Rc::make_mut(&mut self.0).push(value);
    }

    /// Clear the array.
    pub fn clear(&mut self) {
        if Rc::strong_count(&self.0) == 1 {
            Rc::make_mut(&mut self.0).clear();
        } else {
            *self = Self::new();
        }
    }

    /// Iterate over references to the contained values.
    pub fn iter(&self) -> std::slice::Iter<Value> {
        self.0.iter()
    }

    /// Return a sorted version of this array.
    ///
    /// Returns an error if two values could not be compared.
    pub fn sorted(mut self) -> StrResult<Self> {
        let mut result = Ok(());
        Rc::make_mut(&mut self.0).sort_by(|a, b| {
            a.partial_cmp(b).unwrap_or_else(|| {
                if result.is_ok() {
                    result = Err(format!(
                        "cannot compare {} with {}",
                        a.type_name(),
                        b.type_name(),
                    ));
                }
                Ordering::Equal
            })
        });
        result.map(|_| self)
    }

    /// Repeat this array `n` times.
    pub fn repeat(&self, n: i64) -> StrResult<Self> {
        let count = usize::try_from(n)
            .ok()
            .and_then(|n| self.0.len().checked_mul(n))
            .ok_or_else(|| format!("cannot repeat this array {} times", n))?;

        Ok(self.iter().cloned().cycle().take(count).collect())
    }
}

/// The out of bounds access error message.
#[cold]
fn out_of_bounds(index: i64, len: i64) -> String {
    format!("array index out of bounds (index: {}, len: {})", index, len)
}

impl Debug for Array {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_char('(')?;
        for (i, value) in self.iter().enumerate() {
            value.fmt(f)?;
            if i + 1 < self.0.len() {
                f.write_str(", ")?;
            }
        }
        if self.len() == 1 {
            f.write_char(',')?;
        }
        f.write_char(')')
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
        match Rc::try_unwrap(rhs.0) {
            Ok(vec) => self.extend(vec),
            Err(rc) => self.extend(rc.iter().cloned()),
        }
    }
}

impl Extend<Value> for Array {
    fn extend<T: IntoIterator<Item = Value>>(&mut self, iter: T) {
        Rc::make_mut(&mut self.0).extend(iter);
    }
}

impl FromIterator<Value> for Array {
    fn from_iter<T: IntoIterator<Item = Value>>(iter: T) -> Self {
        Self(Rc::new(iter.into_iter().collect()))
    }
}

impl IntoIterator for Array {
    type Item = Value;
    type IntoIter = std::vec::IntoIter<Value>;

    fn into_iter(self) -> Self::IntoIter {
        Rc::take(self.0).into_iter()
    }
}

impl<'a> IntoIterator for &'a Array {
    type Item = &'a Value;
    type IntoIter = std::slice::Iter<'a, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
