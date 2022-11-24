use std::cmp::Ordering;
use std::fmt::{self, Debug, Formatter, Write};
use std::ops::{Add, AddAssign};
use std::sync::Arc;

use super::{ops, Args, Func, Value, Vm};
use crate::diag::{At, SourceResult, StrResult};
use crate::syntax::Spanned;
use crate::util::ArcExt;

/// Create a new [`Array`] from values.
#[macro_export]
#[doc(hidden)]
macro_rules! __array {
    ($value:expr; $count:expr) => {
        $crate::model::Array::from_vec(vec![$value.into(); $count])
    };

    ($($value:expr),* $(,)?) => {
        $crate::model::Array::from_vec(vec![$($value.into()),*])
    };
}

#[doc(inline)]
pub use crate::__array as array;

/// A reference counted array with value semantics.
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

    /// The length of the array.
    pub fn len(&self) -> i64 {
        self.0.len() as i64
    }

    /// The first value in the array.
    pub fn first(&self) -> Option<&Value> {
        self.0.first()
    }

    /// The last value in the array.
    pub fn last(&self) -> Option<&Value> {
        self.0.last()
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
        Arc::make_mut(&mut self.0).pop().ok_or_else(array_is_empty)?;
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
        Ok(())
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

        Ok(Self::from_vec(self.0[start..end].to_vec()))
    }

    /// Whether the array contains a specific value.
    pub fn contains(&self, value: &Value) -> bool {
        self.0.contains(value)
    }

    /// Return the first matching element.
    pub fn find(&self, vm: &Vm, f: Spanned<Func>) -> SourceResult<Option<Value>> {
        for item in self.iter() {
            let args = Args::new(f.span, [item.clone()]);
            if f.v.call(vm, args)?.cast::<bool>().at(f.span)? {
                return Ok(Some(item.clone()));
            }
        }

        Ok(None)
    }

    /// Return the index of the first matching element.
    pub fn position(&self, vm: &Vm, f: Spanned<Func>) -> SourceResult<Option<i64>> {
        for (i, item) in self.iter().enumerate() {
            let args = Args::new(f.span, [item.clone()]);
            if f.v.call(vm, args)?.cast::<bool>().at(f.span)? {
                return Ok(Some(i as i64));
            }
        }

        Ok(None)
    }

    /// Return a new array with only those elements for which the function
    /// returns true.
    pub fn filter(&self, vm: &Vm, f: Spanned<Func>) -> SourceResult<Self> {
        let mut kept = vec![];
        for item in self.iter() {
            let args = Args::new(f.span, [item.clone()]);
            if f.v.call(vm, args)?.cast::<bool>().at(f.span)? {
                kept.push(item.clone())
            }
        }
        Ok(Self::from_vec(kept))
    }

    /// Transform each item in the array with a function.
    pub fn map(&self, vm: &Vm, f: Spanned<Func>) -> SourceResult<Self> {
        let enumerate = f.v.argc() == Some(2);
        self.iter()
            .enumerate()
            .map(|(i, item)| {
                let mut args = Args::new(f.span, []);
                if enumerate {
                    args.push(f.span, Value::Int(i as i64));
                }
                args.push(f.span, item.clone());
                f.v.call(vm, args)
            })
            .collect()
    }

    /// Whether any element matches.
    pub fn any(&self, vm: &Vm, f: Spanned<Func>) -> SourceResult<bool> {
        for item in self.iter() {
            let args = Args::new(f.span, [item.clone()]);
            if f.v.call(vm, args)?.cast::<bool>().at(f.span)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Whether all elements match.
    pub fn all(&self, vm: &Vm, f: Spanned<Func>) -> SourceResult<bool> {
        for item in self.iter() {
            let args = Args::new(f.span, [item.clone()]);
            if !f.v.call(vm, args)?.cast::<bool>().at(f.span)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Return a new array with all items from this and nested arrays.
    pub fn flatten(&self) -> Self {
        let mut flat = Vec::with_capacity(self.0.len());
        for item in self.iter() {
            if let Value::Array(nested) = item {
                flat.extend(nested.flatten().into_iter());
            } else {
                flat.push(item.clone());
            }
        }
        Self::from_vec(flat)
    }

    /// Returns a new array with reversed order.
    pub fn rev(&self) -> Self {
        self.0.iter().cloned().rev().collect()
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
        usize::try_from(if index >= 0 { index } else { self.len().checked_add(index)? })
            .ok()
    }
}

/// The out of bounds access error message.
#[cold]
fn out_of_bounds(index: i64, len: i64) -> String {
    format!("array index out of bounds (index: {}, len: {})", index, len)
}

/// The error message when the array is empty.
#[cold]
fn array_is_empty() -> String {
    "array is empty".into()
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
