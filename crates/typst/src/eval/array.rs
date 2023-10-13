use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::num::NonZeroI64;
use std::ops::{Add, AddAssign};

use ecow::{eco_format, EcoString, EcoVec};
use serde::{Deserialize, Serialize};

use super::{
    cast, func, ops, scope, ty, Args, Bytes, CastInfo, FromValue, Func, IntoValue,
    Reflect, Repr, Value, Version, Vm,
};
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

/// A sequence of values.
///
/// You can construct an array by enclosing a comma-separated sequence of values
/// in parentheses. The values do not have to be of the same type.
///
/// You can access and update array items with the `.at()` method. Indices are
/// zero-based and negative indices wrap around to the end of the array. You can
/// iterate over an array using a [for loop]($scripting/#loops). Arrays can be
/// added together with the `+` operator, [joined together]($scripting/#blocks)
/// and multiplied with integers.
///
/// **Note:** An array of length one needs a trailing comma, as in `{(1,)}`.
/// This is to disambiguate from a simple parenthesized expressions like `{(1 +
/// 2) * 3}`. An empty array is written as `{()}`.
///
/// # Example
/// ```example
/// #let values = (1, 7, 4, -3, 2)
///
/// #values.at(0) \
/// #(values.at(0) = 3)
/// #values.at(-1) \
/// #values.find(calc.even) \
/// #values.filter(calc.odd) \
/// #values.map(calc.abs) \
/// #values.rev() \
/// #(1, (2, 3)).flatten() \
/// #(("A", "B", "C")
///     .join(", ", last: " and "))
/// ```
#[ty(scope)]
#[derive(Default, Clone, PartialEq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Array(EcoVec<Value>);

impl Array {
    /// Create a new, empty array.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new vec, with a known capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self(EcoVec::with_capacity(capacity))
    }

    /// Return `true` if the length is 0.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Extract a slice of the whole array.
    pub fn as_slice(&self) -> &[Value] {
        self.0.as_slice()
    }

    /// Iterate over references to the contained values.
    pub fn iter(&self) -> std::slice::Iter<Value> {
        self.0.iter()
    }

    /// Mutably borrow the first value in the array.
    pub fn first_mut(&mut self) -> StrResult<&mut Value> {
        self.0.make_mut().first_mut().ok_or_else(array_is_empty)
    }

    /// Mutably borrow the last value in the array.
    pub fn last_mut(&mut self) -> StrResult<&mut Value> {
        self.0.make_mut().last_mut().ok_or_else(array_is_empty)
    }

    /// Mutably borrow the value at the given index.
    pub fn at_mut(&mut self, index: i64) -> StrResult<&mut Value> {
        let len = self.len();
        self.locate_opt(index, false)
            .and_then(move |i| self.0.make_mut().get_mut(i))
            .ok_or_else(|| out_of_bounds_no_default(index, len))
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

    /// Repeat this array `n` times.
    pub fn repeat(&self, n: usize) -> StrResult<Self> {
        let count = self
            .len()
            .checked_mul(n)
            .ok_or_else(|| format!("cannot repeat this array {} times", n))?;

        Ok(self.iter().cloned().cycle().take(count).collect())
    }
}

#[scope]
impl Array {
    /// Converts a value to an array.
    ///
    /// Note that this function is only intended for conversion of a collection-like
    /// value to an array, not for creation of an array from individual items. Use
    /// the array syntax `(1, 2, 3)` (or `(1,)` for a single-element array) instead.
    ///
    /// ```example
    /// #let hi = "Hello 😃"
    /// #array(bytes(hi))
    /// ```
    #[func(constructor)]
    pub fn construct(
        /// The value that should be converted to an array.
        value: ToArray,
    ) -> Array {
        value.0
    }

    /// The number of values in the array.
    #[func(title = "Length")]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns the first item in the array. May be used on the left-hand side
    /// of an assignment. Fails with an error if the array is empty.
    #[func]
    pub fn first(&self) -> StrResult<Value> {
        self.0.first().cloned().ok_or_else(array_is_empty)
    }

    /// Returns the last item in the array. May be used on the left-hand side of
    /// an assignment. Fails with an error if the array is empty.
    #[func]
    pub fn last(&self) -> StrResult<Value> {
        self.0.last().cloned().ok_or_else(array_is_empty)
    }

    /// Returns the item at the specified index in the array. May be used on the
    /// left-hand side of an assignment. Returns the default value if the index
    /// is out of bounds or fails with an error if no default value was
    /// specified.
    #[func]
    pub fn at(
        &self,
        /// The index at which to retrieve the item. If negative, indexes from
        /// the back.
        index: i64,
        /// A default value to return if the index is out of bounds.
        #[named]
        default: Option<Value>,
    ) -> StrResult<Value> {
        self.locate_opt(index, false)
            .and_then(|i| self.0.get(i).cloned())
            .or(default)
            .ok_or_else(|| out_of_bounds_no_default(index, self.len()))
    }

    /// Adds a value to the end of the array.
    #[func]
    pub fn push(
        &mut self,
        /// The value to insert at the end of the array.
        value: Value,
    ) {
        self.0.push(value);
    }

    /// Removes the last item from the array and returns it. Fails with an error
    /// if the array is empty.
    #[func]
    pub fn pop(&mut self) -> StrResult<Value> {
        self.0.pop().ok_or_else(array_is_empty)
    }

    /// Inserts a value into the array at the specified index. Fails with an
    /// error if the index is out of bounds.
    #[func]
    pub fn insert(
        &mut self,
        /// The index at which to insert the item. If negative, indexes from
        /// the back.
        index: i64,
        /// The value to insert into the array.
        value: Value,
    ) -> StrResult<()> {
        let i = self.locate(index, true)?;
        self.0.insert(i, value);
        Ok(())
    }

    /// Removes the value at the specified index from the array and return it.
    #[func]
    pub fn remove(
        &mut self,
        /// The index at which to remove the item. If negative, indexes from
        /// the back.
        index: i64,
        /// A default value to return if the index is out of bounds.
        #[named]
        default: Option<Value>,
    ) -> StrResult<Value> {
        self.locate_opt(index, false)
            .map(|i| self.0.remove(i))
            .or(default)
            .ok_or_else(|| out_of_bounds_no_default(index, self.len()))
    }

    /// Extracts a subslice of the array. Fails with an error if the start or
    /// index is out of bounds.
    #[func]
    pub fn slice(
        &self,
        /// The start index (inclusive). If negative, indexes from the back.
        start: i64,
        /// The end index (exclusive). If omitted, the whole slice until the end
        /// of the array is extracted. If negative, indexes from the back.
        #[default]
        end: Option<i64>,
        /// The number of items to extract. This is equivalent to passing
        /// `start + count` as the `end` position. Mutually exclusive with `end`.
        #[named]
        count: Option<i64>,
    ) -> StrResult<Array> {
        let mut end = end;
        if end.is_none() {
            end = count.map(|c: i64| start + c);
        }
        let start = self.locate(start, true)?;
        let end = self.locate(end.unwrap_or(self.len() as i64), true)?.max(start);
        Ok(self.0[start..end].into())
    }

    /// Whether the array contains the specified value.
    ///
    /// This method also has dedicated syntax: You can write `{2 in (1, 2, 3)}`
    /// instead of `{(1, 2, 3).contains(2)}`.
    #[func]
    pub fn contains(
        &self,
        /// The value to search for.
        value: Value,
    ) -> bool {
        self.0.contains(&value)
    }

    /// Searches for an item for which the given function returns `{true}` and
    /// returns the first match or `{none}` if there is no match.
    #[func]
    pub fn find(
        &self,
        /// The virtual machine.
        vm: &mut Vm,
        /// The function to apply to each item. Must return a boolean.
        searcher: Func,
    ) -> SourceResult<Option<Value>> {
        for item in self.iter() {
            let args = Args::new(searcher.span(), [item.clone()]);
            if searcher.call_vm(vm, args)?.cast::<bool>().at(searcher.span())? {
                return Ok(Some(item.clone()));
            }
        }
        Ok(None)
    }

    /// Searches for an item for which the given function returns `{true}` and
    /// returns the index of the first match or `{none}` if there is no match.
    #[func]
    pub fn position(
        &self,
        /// The virtual machine.
        vm: &mut Vm,
        /// The function to apply to each item. Must return a boolean.
        searcher: Func,
    ) -> SourceResult<Option<i64>> {
        for (i, item) in self.iter().enumerate() {
            let args = Args::new(searcher.span(), [item.clone()]);
            if searcher.call_vm(vm, args)?.cast::<bool>().at(searcher.span())? {
                return Ok(Some(i as i64));
            }
        }

        Ok(None)
    }

    /// Create an array consisting of a sequence of numbers.
    ///
    /// If you pass just one positional parameter, it is interpreted as the
    /// `end` of the range. If you pass two, they describe the `start` and `end`
    /// of the range.
    ///
    /// This function is available both in the array function's scope and
    /// globally.
    ///
    /// ```example
    /// #range(5) \
    /// #range(2, 5) \
    /// #range(20, step: 4) \
    /// #range(21, step: 4) \
    /// #range(5, 2, step: -1)
    /// ```
    #[func]
    pub fn range(
        /// The real arguments (the other arguments are just for the docs, this
        /// function is a bit involved, so we parse the arguments manually).
        args: Args,
        /// The start of the range (inclusive).
        #[external]
        #[default]
        start: i64,
        /// The end of the range (exclusive).
        #[external]
        end: i64,
        /// The distance between the generated numbers.
        #[named]
        #[default(NonZeroI64::new(1).unwrap())]
        step: NonZeroI64,
    ) -> SourceResult<Array> {
        let mut args = args;
        let first = args.expect::<i64>("end")?;
        let (start, end) = match args.eat::<i64>()? {
            Some(second) => (first, second),
            None => (0, first),
        };
        args.finish()?;

        let step = step.get();

        let mut x = start;
        let mut array = Self::new();

        while x.cmp(&end) == 0.cmp(&step) {
            array.push(x.into_value());
            x += step;
        }

        Ok(array)
    }

    /// Produces a new array with only the items from the original one for which
    /// the given function returns true.
    #[func]
    pub fn filter(
        &self,
        /// The virtual machine.
        vm: &mut Vm,
        /// The function to apply to each item. Must return a boolean.
        test: Func,
    ) -> SourceResult<Array> {
        let mut kept = EcoVec::new();
        for item in self.iter() {
            let args = Args::new(test.span(), [item.clone()]);
            if test.call_vm(vm, args)?.cast::<bool>().at(test.span())? {
                kept.push(item.clone())
            }
        }
        Ok(kept.into())
    }

    /// Produces a new array in which all items from the original one were
    /// transformed with the given function.
    #[func]
    pub fn map(
        &self,
        /// The virtual machine.
        vm: &mut Vm,
        /// The function to apply to each item.
        mapper: Func,
    ) -> SourceResult<Array> {
        self.iter()
            .map(|item| {
                let args = Args::new(mapper.span(), [item.clone()]);
                mapper.call_vm(vm, args)
            })
            .collect()
    }

    /// Returns a new array with the values alongside their indices.
    ///
    /// The returned array consists of `(index, value)` pairs in the form of
    /// length-2 arrays. These can be [destructured]($scripting/#bindings) with
    /// a let binding or for loop.
    #[func]
    pub fn enumerate(
        &self,
        /// The index returned for the first pair of the returned list.
        #[named]
        #[default(0)]
        start: i64,
    ) -> StrResult<Array> {
        self.iter()
            .enumerate()
            .map(|(i, value)| {
                Ok(array![
                    start
                        .checked_add_unsigned(i as u64)
                        .ok_or("array index is too large")?,
                    value.clone()
                ]
                .into_value())
            })
            .collect()
    }

    /// Zips the array with other arrays. If the arrays are of unequal length,
    /// it will only zip up until the last element of the shortest array and the
    /// remaining elements will be ignored. The return value is an array where
    /// each element is yet another array, the size of each of those is the
    /// number of zipped arrays.
    ///
    /// This function is variadic, meaning that you can zip multiple arrays
    /// together at once: `{(1, 2, 3).zip((3, 4, 5), (6, 7, 8))}` yields
    /// `{((1, 3, 6), (2, 4, 7), (3, 5, 8))}`.
    #[func]
    pub fn zip(
        &self,
        /// The real arguments (the other arguments are just for the docs, this
        /// function is a bit involved, so we parse the arguments manually).
        args: Args,
        /// The arrays to zip with.
        #[external]
        #[variadic]
        others: Vec<Array>,
    ) -> SourceResult<Array> {
        // Fast path for just two arrays.
        let mut args = args;
        if args.remaining() <= 1 {
            let other = args.expect::<Array>("others")?;
            args.finish()?;
            return Ok(self
                .iter()
                .zip(other)
                .map(|(first, second)| array![first.clone(), second].into_value())
                .collect());
        }

        // If there is more than one array, we use the manual method.
        let mut out = Self::with_capacity(self.len());
        let mut iterators = args
            .all::<Array>()?
            .into_iter()
            .map(|i| i.into_iter())
            .collect::<Vec<_>>();
        args.finish()?;

        for this in self.iter() {
            let mut row = Self::with_capacity(1 + iterators.len());
            row.push(this.clone());

            for iterator in &mut iterators {
                let Some(item) = iterator.next() else {
                    return Ok(out);
                };

                row.push(item);
            }

            out.push(row.into_value());
        }

        Ok(out)
    }

    /// Folds all items into a single value using an accumulator function.
    #[func]
    pub fn fold(
        &self,
        /// The virtual machine.
        vm: &mut Vm,
        /// The initial value to start with.
        init: Value,
        /// The folding function. Must have two parameters: One for the
        /// accumulated value and one for an item.
        folder: Func,
    ) -> SourceResult<Value> {
        let mut acc = init;
        for item in self.iter() {
            let args = Args::new(folder.span(), [acc, item.clone()]);
            acc = folder.call_vm(vm, args)?;
        }
        Ok(acc)
    }

    /// Sums all items (works for all types that can be added).
    #[func]
    pub fn sum(
        &self,
        /// What to return if the array is empty. Must be set if the array can
        /// be empty.
        #[named]
        default: Option<Value>,
    ) -> StrResult<Value> {
        let mut acc = self
            .0
            .first()
            .cloned()
            .or(default)
            .ok_or("cannot calculate sum of empty array with no default")?;
        for i in self.iter().skip(1) {
            acc = add(acc, i.clone())?;
        }
        Ok(acc)
    }

    /// Calculates the product all items (works for all types that can be
    /// multiplied).
    #[func]
    pub fn product(
        &self,
        /// What to return if the array is empty. Must be set if the array can
        /// be empty.
        #[named]
        default: Option<Value>,
    ) -> StrResult<Value> {
        let mut acc = self
            .0
            .first()
            .cloned()
            .or(default)
            .ok_or("cannot calculate product of empty array with no default")?;
        for i in self.iter().skip(1) {
            acc = mul(acc, i.clone())?;
        }
        Ok(acc)
    }

    /// Whether the given function returns `{true}` for any item in the array.
    #[func]
    pub fn any(
        &self,
        /// The virtual machine.
        vm: &mut Vm,
        /// The function to apply to each item. Must return a boolean.
        test: Func,
    ) -> SourceResult<bool> {
        for item in self.iter() {
            let args = Args::new(test.span(), [item.clone()]);
            if test.call_vm(vm, args)?.cast::<bool>().at(test.span())? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Whether the given function returns `{true}` for all items in the array.
    #[func]
    pub fn all(
        &self,
        /// The virtual machine.
        vm: &mut Vm,
        /// The function to apply to each item. Must return a boolean.
        test: Func,
    ) -> SourceResult<bool> {
        for item in self.iter() {
            let args = Args::new(test.span(), [item.clone()]);
            if !test.call_vm(vm, args)?.cast::<bool>().at(test.span())? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Combine all nested arrays into a single flat one.
    #[func]
    pub fn flatten(&self) -> Array {
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

    /// Return a new array with the same items, but in reverse order.
    #[func(title = "Reverse")]
    pub fn rev(&self) -> Array {
        self.0.iter().cloned().rev().collect()
    }

    /// Split the array at occurrences of the specified value.
    #[func]
    pub fn split(
        &self,
        /// The value to split at.
        at: Value,
    ) -> Array {
        self.as_slice()
            .split(|value| *value == at)
            .map(|subslice| Value::Array(subslice.iter().cloned().collect()))
            .collect()
    }

    /// Combine all items in the array into one.
    #[func]
    pub fn join(
        &self,
        /// A value to insert between each item of the array.
        #[default]
        separator: Option<Value>,
        /// An alternative separator between the last two items.
        #[named]
        last: Option<Value>,
    ) -> StrResult<Value> {
        let len = self.0.len();
        let separator = separator.unwrap_or(Value::None);

        let mut last = last;
        let mut result = Value::None;
        for (i, value) in self.iter().cloned().enumerate() {
            if i > 0 {
                if i + 1 == len && last.is_some() {
                    result = ops::join(result, last.take().unwrap())?;
                } else {
                    result = ops::join(result, separator.clone())?;
                }
            }

            result = ops::join(result, value)?;
        }

        Ok(result)
    }

    /// Returns an array with a copy of the separator value placed between
    /// adjacent elements.
    #[func]
    pub fn intersperse(
        &self,
        /// The value that will be placed between each adjacent element.
        separator: Value,
    ) -> Array {
        // TODO: Use once stabilized:
        // https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.intersperse
        let size = match self.len() {
            0 => return Array::new(),
            n => (2 * n) - 1,
        };
        let mut vec = EcoVec::with_capacity(size);
        let mut iter = self.iter().cloned();

        if let Some(first) = iter.next() {
            vec.push(first);
        }

        for value in iter {
            vec.push(separator.clone());
            vec.push(value);
        }

        Array(vec)
    }

    /// Return a sorted version of this array, optionally by a given key
    /// function. The sorting algorithm used is stable.
    ///
    /// Returns an error if two values could not be compared or if the key
    /// function (if given) yields an error.
    #[func]
    pub fn sorted(
        &self,
        /// The virtual machine.
        vm: &mut Vm,
        /// The callsite span.
        span: Span,
        /// If given, applies this function to the elements in the array to
        /// determine the keys to sort by.
        #[named]
        key: Option<Func>,
    ) -> SourceResult<Array> {
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

    /// Deduplicates all items in the array.
    ///
    /// Returns a new array with all duplicate items removed. Only the first
    /// element of each duplicate is kept.
    ///
    /// ```example
    /// #(1, 1, 2, 3, 1).dedup()
    /// ```
    #[func(title = "Deduplicate")]
    pub fn dedup(
        &self,
        /// The virtual machine.
        vm: &mut Vm,
        /// If given, applies this function to the elements in the array to
        /// determine the keys to deduplicate by.
        #[named]
        key: Option<Func>,
    ) -> SourceResult<Array> {
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
}

/// A value that can be cast to bytes.
pub struct ToArray(Array);

cast! {
    ToArray,
    v: Bytes => Self(v.iter().map(|&b| Value::Int(b.into())).collect()),
    v: Array => Self(v),
    v: Version => Self(v.values().iter().map(|&v| Value::Int(v as i64)).collect())
}

impl Debug for Array {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_list().entries(&self.0).finish()
    }
}

impl Repr for Array {
    fn repr(&self) -> EcoString {
        let max = 40;
        let mut pieces: Vec<_> = self
            .iter()
            .take(max)
            .map(|value| eco_format!("{}", value.repr()))
            .collect();
        if self.len() > max {
            pieces.push(eco_format!(".. ({} items omitted)", self.len() - max));
        }
        pretty_array_like(&pieces, self.len() == 1).into()
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
    fn add_assign(&mut self, rhs: Self) {
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
    fn input() -> CastInfo {
        Array::input()
    }

    fn output() -> CastInfo {
        Array::output()
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
