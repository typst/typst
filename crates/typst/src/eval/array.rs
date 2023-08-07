use std::cmp::Ordering;
use std::fmt::{self, Debug, Formatter};
use std::ops::{Add, AddAssign};

use ecow::{eco_format, EcoString, EcoVec};
use serde::Serialize;

use super::{ops, Args, CastInfo, FromValue, Func, IntoValue, Reflect, Value, Vm};
use crate::diag::{At, SourceResult, StrResult};
use crate::eval::ops::{add, mul};
use crate::syntax::Span;
use crate::util::pretty_array_like;

/// Create a new [`Array`] from values.
#[macro_export]
#[doc(hidden)]
macro_rules! __array {
    ($value:expr; $count:expr) => {
        $crate::eval::Array::from($crate::eval::eco_vec![
            $crate::eval::IntoValue::into_value($value);
            $count
        ])
    };

    ($($value:expr),* $(,)?) => {
        $crate::eval::Array::from($crate::eval::eco_vec![$(
            $crate::eval::IntoValue::into_value($value)
        ),*])
    };
}

#[doc(inline)]
pub use crate::__array as array;
#[doc(hidden)]
pub use ecow::eco_vec;

/// A reference counted array with value semantics.
#[derive(Default, Clone, PartialEq, Hash, Serialize)]
pub struct Array(EcoVec<Value>);

impl Array {
    /// Create a new, empty array.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return `true` if the length is 0.
    pub fn is_empty(&self) -> bool {
        self.0.len() == 0
    }

    /// The length of the array.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// The first value in the array.
    pub fn first(&self) -> StrResult<&Value> {
        self.0.first().ok_or_else(array_is_empty)
    }

    /// Mutably borrow the first value in the array.
    pub fn first_mut(&mut self) -> StrResult<&mut Value> {
        self.0.make_mut().first_mut().ok_or_else(array_is_empty)
    }

    /// The last value in the array.
    pub fn last(&self) -> StrResult<&Value> {
        self.0.last().ok_or_else(array_is_empty)
    }

    /// Mutably borrow the last value in the array.
    pub fn last_mut(&mut self) -> StrResult<&mut Value> {
        self.0.make_mut().last_mut().ok_or_else(array_is_empty)
    }

    /// Borrow the value at the given index.
    pub fn at(&self, index: i64, default: Option<Value>) -> StrResult<Value> {
        self.locate_opt(index, false)
            .and_then(|i| self.0.get(i).cloned())
            .or(default)
            .ok_or_else(|| out_of_bounds_no_default(index, self.len()))
    }

    /// Mutably borrow the value at the given index.
    pub fn at_mut(&mut self, index: i64) -> StrResult<&mut Value> {
        let len = self.len();
        self.locate_opt(index, false)
            .and_then(move |i| self.0.make_mut().get_mut(i))
            .ok_or_else(|| out_of_bounds_no_default(index, len))
    }

    /// Push a value to the end of the array.
    pub fn push(&mut self, value: Value) {
        self.0.push(value);
    }

    /// Remove the last value in the array.
    pub fn pop(&mut self) -> StrResult<Value> {
        self.0.pop().ok_or_else(array_is_empty)
    }

    /// Insert a value at the specified index.
    pub fn insert(&mut self, index: i64, value: Value) -> StrResult<()> {
        let i = self.locate(index, true)?;
        self.0.insert(i, value);
        Ok(())
    }

    /// Remove and return the value at the specified index.
    pub fn remove(&mut self, index: i64) -> StrResult<Value> {
        let i = self.locate(index, false)?;
        Ok(self.0.remove(i))
    }

    /// Extract a contiguous subregion of the array.
    pub fn slice(&self, start: i64, end: Option<i64>) -> StrResult<Self> {
        let start = self.locate(start, true)?;
        let end = self.locate(end.unwrap_or(self.len() as i64), true)?.max(start);
        Ok(self.0[start..end].into())
    }

    /// Whether the array contains a specific value.
    pub fn contains(&self, value: &Value) -> bool {
        self.0.contains(value)
    }

    /// Return the first matching item.
    pub fn find(&self, vm: &mut Vm, func: Func) -> SourceResult<Option<Value>> {
        for item in self.iter() {
            let args = Args::new(func.span(), [item.clone()]);
            if func.call_vm(vm, args)?.cast::<bool>().at(func.span())? {
                return Ok(Some(item.clone()));
            }
        }
        Ok(None)
    }

    /// Return the index of the first matching item.
    pub fn position(&self, vm: &mut Vm, func: Func) -> SourceResult<Option<i64>> {
        for (i, item) in self.iter().enumerate() {
            let args = Args::new(func.span(), [item.clone()]);
            if func.call_vm(vm, args)?.cast::<bool>().at(func.span())? {
                return Ok(Some(i as i64));
            }
        }

        Ok(None)
    }

    /// Return a new array with only those items for which the function returns
    /// true.
    pub fn filter(&self, vm: &mut Vm, func: Func) -> SourceResult<Self> {
        let mut kept = EcoVec::new();
        for item in self.iter() {
            let args = Args::new(func.span(), [item.clone()]);
            if func.call_vm(vm, args)?.cast::<bool>().at(func.span())? {
                kept.push(item.clone())
            }
        }
        Ok(kept.into())
    }

    /// Transform each item in the array with a function.
    pub fn map(&self, vm: &mut Vm, func: Func) -> SourceResult<Self> {
        self.iter()
            .map(|item| {
                let args = Args::new(func.span(), [item.clone()]);
                func.call_vm(vm, args)
            })
            .collect()
    }

    /// Fold all of the array's items into one with a function.
    pub fn fold(&self, vm: &mut Vm, init: Value, func: Func) -> SourceResult<Value> {
        let mut acc = init;
        for item in self.iter() {
            let args = Args::new(func.span(), [acc, item.clone()]);
            acc = func.call_vm(vm, args)?;
        }
        Ok(acc)
    }

    /// Calculates the sum of the array's items
    pub fn sum(&self, default: Option<Value>, span: Span) -> SourceResult<Value> {
        let mut acc = self
            .first()
            .map(|x| x.clone())
            .or_else(|_| {
                default.ok_or_else(|| {
                    eco_format!("cannot calculate sum of empty array with no default")
                })
            })
            .at(span)?;
        for i in self.iter().skip(1) {
            acc = add(acc, i.clone()).at(span)?;
        }
        Ok(acc)
    }

    /// Calculates the product of the array's items
    pub fn product(&self, default: Option<Value>, span: Span) -> SourceResult<Value> {
        let mut acc = self
            .first()
            .map(|x| x.clone())
            .or_else(|_| {
                default.ok_or_else(|| {
                    eco_format!("cannot calculate product of empty array with no default")
                })
            })
            .at(span)?;
        for i in self.iter().skip(1) {
            acc = mul(acc, i.clone()).at(span)?;
        }
        Ok(acc)
    }

    /// Whether any item matches.
    pub fn any(&self, vm: &mut Vm, func: Func) -> SourceResult<bool> {
        for item in self.iter() {
            let args = Args::new(func.span(), [item.clone()]);
            if func.call_vm(vm, args)?.cast::<bool>().at(func.span())? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Whether all items match.
    pub fn all(&self, vm: &mut Vm, func: Func) -> SourceResult<bool> {
        for item in self.iter() {
            let args = Args::new(func.span(), [item.clone()]);
            if !func.call_vm(vm, args)?.cast::<bool>().at(func.span())? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Return a new array with all items from this and nested arrays.
    pub fn flatten(&self) -> Self {
        let mut flat = EcoVec::with_capacity(self.0.len());
        for item in self.iter() {
            if let Value::Array(nested) = item {
                flat.extend(nested.flatten().into_iter());
            } else {
                flat.push(item.clone());
            }
        }
        flat.into()
    }

    /// Returns a new array with reversed order.
    pub fn rev(&self) -> Self {
        self.0.iter().cloned().rev().collect()
    }

    /// Split all values in the array.
    pub fn split(&self, at: Value) -> Array {
        self.as_slice()
            .split(|value| *value == at)
            .map(|subslice| Value::Array(subslice.iter().cloned().collect()))
            .collect()
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

    /// Zips the array with another array. If the two arrays are of unequal length, it will only
    /// zip up until the last element of the smaller array and the remaining elements will be
    /// ignored. The return value is an array where each element is yet another array of size 2.
    pub fn zip(&self, other: Array) -> Array {
        self.iter()
            .zip(other)
            .map(|(first, second)| array![first.clone(), second].into_value())
            .collect()
    }

    /// Return a sorted version of this array, optionally by a given key function.
    ///
    /// Returns an error if two values could not be compared or if the key function (if given)
    /// yields an error.
    pub fn sorted(
        &self,
        vm: &mut Vm,
        span: Span,
        key: Option<Func>,
    ) -> SourceResult<Self> {
        let mut result = Ok(());
        let mut vec = self.0.clone();
        let mut key_of = |x: Value| match &key {
            // NOTE: We are relying on `comemo`'s memoization of function
            // evaluation to not excessively reevaluate the `key`.
            Some(f) => f.call_vm(vm, Args::new(f.span(), [x])),
            None => Ok(x),
        };
        vec.make_mut().sort_by(|a, b| {
            // Until we get `try` blocks :)
            match (key_of(a.clone()), key_of(b.clone())) {
                (Ok(a), Ok(b)) => {
                    typst::eval::ops::compare(&a, &b).unwrap_or_else(|err| {
                        if result.is_ok() {
                            result = Err(err).at(span);
                        }
                        Ordering::Equal
                    })
                }
                (Err(e), _) | (_, Err(e)) => {
                    if result.is_ok() {
                        result = Err(e);
                    }
                    Ordering::Equal
                }
            }
        });
        result.map(|_| vec.into())
    }

    /// Repeat this array `n` times.
    pub fn repeat(&self, n: i64) -> StrResult<Self> {
        let count = usize::try_from(n)
            .ok()
            .and_then(|n| self.0.len().checked_mul(n))
            .ok_or_else(|| format!("cannot repeat this array {} times", n))?;

        Ok(self.iter().cloned().cycle().take(count).collect())
    }

    /// Enumerate all items in the array.
    pub fn enumerate(&self, start: i64) -> StrResult<Self> {
        self.iter()
            .enumerate()
            .map(|(i, value)| {
                Ok(array![
                    start
                        .checked_add_unsigned(i as u64)
                        .ok_or_else(|| "array index is too large".to_string())?,
                    value.clone()
                ]
                .into_value())
            })
            .collect()
    }

    /// Deduplicates all items in the array.
    pub fn dedup(&self, vm: &mut Vm, key: Option<Func>) -> SourceResult<Self> {
        let mut out = EcoVec::with_capacity(self.0.len());
        let mut key_of = |x: Value| match &key {
            // NOTE: We are relying on `comemo`'s memoization of function
            // evaluation to not excessively reevaluate the `key`.
            Some(f) => f.call_vm(vm, Args::new(f.span(), [x])),
            None => Ok(x),
        };

        // This algorithm is O(N^2) because we cannot rely on `HashSet` since:
        // 1. We would like to preserve the order of the elements.
        // 2. We cannot hash arbitrary `Value`.
        'outer: for value in self.iter() {
            let key = key_of(value.clone())?;
            if out.is_empty() {
                out.push(value.clone());
                continue;
            }

            for second in out.iter() {
                if typst::eval::ops::equal(&key, &key_of(second.clone())?) {
                    continue 'outer;
                }
            }

            out.push(value.clone());
        }

        Ok(Self(out))
    }

    /// Extract a slice of the whole array.
    pub fn as_slice(&self) -> &[Value] {
        self.0.as_slice()
    }

    /// Iterate over references to the contained values.
    pub fn iter(&self) -> std::slice::Iter<Value> {
        self.0.iter()
    }

    /// Resolve an index or throw an out of bounds error.
    fn locate(&self, index: i64, end_ok: bool) -> StrResult<usize> {
        self.locate_opt(index, end_ok)
            .ok_or_else(|| out_of_bounds(index, self.len()))
    }

    /// Resolve an index, if it is within bounds.
    ///
    /// `index == len` is considered in bounds if and only if `end_ok` is true.
    fn locate_opt(&self, index: i64, end_ok: bool) -> Option<usize> {
        let wrapped =
            if index >= 0 { Some(index) } else { (self.len() as i64).checked_add(index) };

        wrapped
            .and_then(|v| usize::try_from(v).ok())
            .filter(|&v| v < self.0.len() + end_ok as usize)
    }
}

impl Debug for Array {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let max = 40;
        let mut pieces: Vec<_> =
            self.iter().take(max).map(|value| eco_format!("{value:?}")).collect();
        if self.len() > max {
            pieces.push(eco_format!(".. ({} items omitted)", self.len() - max));
        }
        f.write_str(&pretty_array_like(&pieces, self.len() == 1))
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
        self.0.extend(rhs.0);
    }
}

impl Extend<Value> for Array {
    fn extend<T: IntoIterator<Item = Value>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}

impl FromIterator<Value> for Array {
    fn from_iter<T: IntoIterator<Item = Value>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl IntoIterator for Array {
    type Item = Value;
    type IntoIter = ecow::vec::IntoIter<Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Array {
    type Item = &'a Value;
    type IntoIter = std::slice::Iter<'a, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl From<EcoVec<Value>> for Array {
    fn from(v: EcoVec<Value>) -> Self {
        Array(v)
    }
}

impl From<&[Value]> for Array {
    fn from(v: &[Value]) -> Self {
        Array(v.into())
    }
}

impl<T> Reflect for Vec<T> {
    fn describe() -> CastInfo {
        Array::describe()
    }

    fn castable(value: &Value) -> bool {
        Array::castable(value)
    }
}

impl<T: IntoValue> IntoValue for Vec<T> {
    fn into_value(self) -> Value {
        Value::Array(self.into_iter().map(IntoValue::into_value).collect())
    }
}

impl<T: FromValue> FromValue for Vec<T> {
    fn from_value(value: Value) -> StrResult<Self> {
        value.cast::<Array>()?.into_iter().map(Value::cast).collect()
    }
}

/// The error message when the array is empty.
#[cold]
fn array_is_empty() -> EcoString {
    "array is empty".into()
}

/// The out of bounds access error message.
#[cold]
fn out_of_bounds(index: i64, len: usize) -> EcoString {
    eco_format!("array index out of bounds (index: {index}, len: {len})")
}

/// The out of bounds access error message when no default value was given.
#[cold]
fn out_of_bounds_no_default(index: i64, len: usize) -> EcoString {
    eco_format!(
        "array index out of bounds (index: {index}, len: {len}) \
         and no default value was specified",
    )
}
