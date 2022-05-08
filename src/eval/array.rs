use std::cmp::Ordering;
use std::fmt::{self, Debug, Formatter, Write};
use std::ops::{Add, AddAssign};
use std::sync::Arc;

use super::{ops, Args, Func, Value};
use crate::diag::{At, StrResult, TypResult};
use crate::syntax::Spanned;
use crate::util::ArcExt;
use crate::Context;

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
#[derive(Default, Clone, PartialEq, Hash)]
pub struct Array(Arc<Vec<Value>>);

impl Array {
    /// Create a new, empty array.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new array from a vector of values.
    pub fn from_vec(vec: Vec<Value>) -> Self {
        Self(Arc::new(vec))
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
        self.locate(index)
            .and_then(|i| self.0.get(i))
            .ok_or_else(|| out_of_bounds(index, self.len()))
    }

    /// Mutably borrow the value at the given index.
    pub fn get_mut(&mut self, index: i64) -> StrResult<&mut Value> {
        let len = self.len();
        self.locate(index)
            .and_then(move |i| Arc::make_mut(&mut self.0).get_mut(i))
            .ok_or_else(|| out_of_bounds(index, len))
    }

    /// Push a value to the end of the array.
    pub fn push(&mut self, value: Value) {
        Arc::make_mut(&mut self.0).push(value);
    }

    /// Remove the last value in the array.
    pub fn pop(&mut self) -> StrResult<()> {
        Arc::make_mut(&mut self.0).pop().ok_or_else(|| "array is empty")?;
        Ok(())
    }

    /// Insert a value at the specified index.
    pub fn insert(&mut self, index: i64, value: Value) -> StrResult<()> {
        let len = self.len();
        let i = self
            .locate(index)
            .filter(|&i| i <= self.0.len())
            .ok_or_else(|| out_of_bounds(index, len))?;

        Arc::make_mut(&mut self.0).insert(i, value);
        Ok(())
    }

    /// Remove and return the value at the specified index.
    pub fn remove(&mut self, index: i64) -> StrResult<()> {
        let len = self.len();
        let i = self
            .locate(index)
            .filter(|&i| i < self.0.len())
            .ok_or_else(|| out_of_bounds(index, len))?;

        Arc::make_mut(&mut self.0).remove(i);
        return Ok(());
    }

    /// Whether the array contains a specific value.
    pub fn contains(&self, value: &Value) -> bool {
        self.0.contains(value)
    }

    /// Extract a contigous subregion of the array.
    pub fn slice(&self, start: i64, end: Option<i64>) -> StrResult<Self> {
        let len = self.len();
        let start = self
            .locate(start)
            .filter(|&start| start <= self.0.len())
            .ok_or_else(|| out_of_bounds(start, len))?;

        let end = end.unwrap_or(self.len());
        let end = self
            .locate(end)
            .filter(|&end| end <= self.0.len())
            .ok_or_else(|| out_of_bounds(end, len))?
            .max(start);

        Ok(Self::from_vec(self.0[start .. end].to_vec()))
    }

    /// Transform each item in the array with a function.
    pub fn map(&self, ctx: &mut Context, f: Spanned<Func>) -> TypResult<Self> {
        Ok(self
            .iter()
            .cloned()
            .map(|item| f.v.call(ctx, Args::from_values(f.span, [item])))
            .collect::<TypResult<_>>()?)
    }

    /// Return a new array with only those elements for which the function
    /// return true.
    pub fn filter(&self, ctx: &mut Context, f: Spanned<Func>) -> TypResult<Self> {
        let mut kept = vec![];
        for item in self.iter() {
            if f.v
                .call(ctx, Args::from_values(f.span, [item.clone()]))?
                .cast::<bool>()
                .at(f.span)?
            {
                kept.push(item.clone())
            }
        }
        Ok(Self::from_vec(kept))
    }

    /// Return a new array with all items from this and nested arrays.
    pub fn flatten(&self) -> Self {
        let mut flat = vec![];
        for item in self.iter() {
            if let Value::Array(nested) = item {
                flat.extend(nested.flatten().into_iter());
            } else {
                flat.push(item.clone());
            }
        }
        Self::from_vec(flat)
    }

    /// Return the index of the element if it is part of the array.
    pub fn find(&self, value: Value) -> Option<i64> {
        self.0.iter().position(|x| *x == value).map(|i| i as i64)
    }

    /// Join all values in the array, optionally with separator and last
    /// separator (between the final two items).
    pub fn join(&self, sep: Option<Value>, mut last: Option<Value>) -> StrResult<Value> {
        let len = self.0.len();
        let sep = sep.unwrap_or(Value::None);

        let mut result = Value::None;
        for (i, value) in self.iter().cloned().enumerate() {
            if i > 0 {
                if i + 1 == len && last.is_some() {
                    result = ops::join(result, last.take().unwrap())?;
                } else {
                    result = ops::join(result, sep.clone())?;
                }
            }

            result = ops::join(result, value)?;
        }

        Ok(result)
    }

    /// Return a sorted version of this array.
    ///
    /// Returns an error if two values could not be compared.
    pub fn sorted(&self) -> StrResult<Self> {
        let mut result = Ok(());
        let mut vec = (*self.0).clone();
        vec.sort_by(|a, b| {
            a.partial_cmp(b).unwrap_or_else(|| {
                if result.is_ok() {
                    result = Err(format!(
                        "cannot order {} and {}",
                        a.type_name(),
                        b.type_name(),
                    ));
                }
                Ordering::Equal
            })
        });
        result.map(|_| Self::from_vec(vec))
    }

    /// Repeat this array `n` times.
    pub fn repeat(&self, n: i64) -> StrResult<Self> {
        let count = usize::try_from(n)
            .ok()
            .and_then(|n| self.0.len().checked_mul(n))
            .ok_or_else(|| format!("cannot repeat this array {} times", n))?;

        Ok(self.iter().cloned().cycle().take(count).collect())
    }

    /// Extract a slice of the whole array.
    pub fn as_slice(&self) -> &[Value] {
        self.0.as_slice()
    }

    /// Iterate over references to the contained values.
    pub fn iter(&self) -> std::slice::Iter<Value> {
        self.0.iter()
    }

    /// Resolve an index.
    fn locate(&self, index: i64) -> Option<usize> {
        usize::try_from(if index >= 0 {
            index
        } else {
            self.len().checked_add(index)?
        })
        .ok()
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
        match Arc::try_unwrap(rhs.0) {
            Ok(vec) => self.extend(vec),
            Err(rc) => self.extend(rc.iter().cloned()),
        }
    }
}

impl Extend<Value> for Array {
    fn extend<T: IntoIterator<Item = Value>>(&mut self, iter: T) {
        Arc::make_mut(&mut self.0).extend(iter);
    }
}

impl FromIterator<Value> for Array {
    fn from_iter<T: IntoIterator<Item = Value>>(iter: T) -> Self {
        Self(Arc::new(iter.into_iter().collect()))
    }
}

impl IntoIterator for Array {
    type Item = Value;
    type IntoIter = std::vec::IntoIter<Value>;

    fn into_iter(self) -> Self::IntoIter {
        Arc::take(self.0).into_iter()
    }
}

impl<'a> IntoIterator for &'a Array {
    type Item = &'a Value;
    type IntoIter = std::slice::Iter<'a, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
