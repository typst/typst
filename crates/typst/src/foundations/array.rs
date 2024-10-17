use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::num::{NonZeroI64, NonZeroUsize};
use std::ops::{Add, AddAssign};

use comemo::Tracked;
use ecow::{eco_format, EcoString, EcoVec};
use rayon::iter::Either;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::diag::{bail, At, HintedStrResult, SourceDiagnostic, SourceResult, StrResult};
use crate::engine::Engine;
use crate::eval::ops;
use crate::foundations::{
    cast, func, repr, scope, ty, Args, Bytes, CastInfo, Context, Dict, FromValue, Func,
    IntoValue, Reflect, Repr, Str, Value, Version,
};
use crate::syntax::{Span, Spanned};

/// Create a new [`Array`] from values.
#[macro_export]
#[doc(hidden)]
macro_rules! __array {
    ($value:expr; $count:expr) => {
        $crate::foundations::Array::from($crate::foundations::eco_vec![
            $crate::foundations::IntoValue::into_value($value);
            $count
        ])
    };

    ($($value:expr),* $(,)?) => {
        $crate::foundations::Array::from($crate::foundations::eco_vec![$(
            $crate::foundations::IntoValue::into_value($value)
        ),*])
    };
}

#[doc(inline)]
pub use crate::__array as array;

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
#[ty(scope, cast)]
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
            .ok_or_else(|| out_of_bounds(index, len))
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
            .ok_or_else(|| format!("cannot repeat this array {n} times"))?;

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

    /// Inserts a value into the array at the specified index, shifting all
    /// subsequent elements to the right. Fails with an error if the index is
    /// out of bounds.
    ///
    /// To replace an element of an array, use [`at`]($array.at).
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

    /// Extracts a subslice of the array. Fails with an error if the start or end
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
        /// The engine.
        engine: &mut Engine,
        /// The callsite context.
        context: Tracked<Context>,
        /// The function to apply to each item. Must return a boolean.
        searcher: Func,
        /// Whether to look for a match in the reversed order.
        #[named]
        #[default(false)]
        rev: bool,
    ) -> SourceResult<Option<Value>> {
        let iter = if rev {
            Either::Right(self.iter().rev())
        } else {
            Either::Left(self.iter())
        };

        for item in iter {
            if searcher
                .call(engine, context, [item.clone()])?
                .cast::<bool>()
                .at(searcher.span())?
            {
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
        /// The engine.
        engine: &mut Engine,
        /// The callsite context.
        context: Tracked<Context>,
        /// The function to apply to each item. Must return a boolean.
        searcher: Func,
        /// Whether to look for a match in the reversed order.
        #[named]
        #[default(false)]
        rev: bool,
    ) -> SourceResult<Option<i64>> {
        let iter = if rev {
            Either::Right(self.iter().enumerate().rev())
        } else {
            Either::Left(self.iter().enumerate())
        };
        for (i, item) in iter {
            if searcher
                .call(engine, context, [item.clone()])?
                .cast::<bool>()
                .at(searcher.span())?
            {
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
        args: &mut Args,
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
        let first = args.expect::<i64>("end")?;
        let (start, end) = match args.eat::<i64>()? {
            Some(second) => (first, second),
            None => (0, first),
        };

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
        /// The engine.
        engine: &mut Engine,
        /// The callsite context.
        context: Tracked<Context>,
        /// The function to apply to each item. Must return a boolean.
        test: Func,
    ) -> SourceResult<Array> {
        let mut kept = EcoVec::new();
        for item in self.iter() {
            if test
                .call(engine, context, [item.clone()])?
                .cast::<bool>()
                .at(test.span())?
            {
                kept.push(item.clone())
            }
        }
        Ok(kept.into())
    }

    /// Produces a new array in which all items from the original one were
    /// transformed with the given function.
    #[func]
    pub fn map(
        self,
        /// The engine.
        engine: &mut Engine,
        /// The callsite context.
        context: Tracked<Context>,
        /// The function to apply to each item.
        mapper: Func,
    ) -> SourceResult<Array> {
        self.into_iter()
            .map(|item| mapper.call(engine, context, [item]))
            .collect()
    }

    /// Returns a new array with the values alongside their indices.
    ///
    /// The returned array consists of `(index, value)` pairs in the form of
    /// length-2 arrays. These can be [destructured]($scripting/#bindings) with
    /// a let binding or for loop.
    #[func]
    pub fn enumerate(
        self,
        /// The index returned for the first pair of the returned list.
        #[named]
        #[default(0)]
        start: i64,
    ) -> StrResult<Array> {
        self.into_iter()
            .enumerate()
            .map(|(i, value)| {
                Ok(array![
                    start
                        .checked_add_unsigned(i as u64)
                        .ok_or("array index is too large")?,
                    value
                ]
                .into_value())
            })
            .collect()
    }

    /// Zips the array with other arrays.
    ///
    /// Returns an array of arrays, where the `i`th inner array contains all the
    /// `i`th elements from each original array.
    ///
    /// If the arrays to be zipped have different lengths, they are zipped up to
    /// the last element of the shortest array and all remaining elements are
    /// ignored.
    ///
    /// This function is variadic, meaning that you can zip multiple arrays
    /// together at once: `{(1, 2).zip(("A", "B"), (10, 20))}` yields
    /// `{((1, "A", 10), (2, "B", 20))}`.
    #[func]
    pub fn zip(
        self,
        /// The real arguments (the `others` arguments are just for the docs, this
        /// function is a bit involved, so we parse the positional arguments manually).
        args: &mut Args,
        /// Whether all arrays have to have the same length.
        /// For example, `{(1, 2).zip((1, 2, 3), exact: true)}` produces an
        /// error.
        #[named]
        #[default(false)]
        exact: bool,
        /// The arrays to zip with.
        #[external]
        #[variadic]
        others: Vec<Array>,
    ) -> SourceResult<Array> {
        let remaining = args.remaining();

        // Fast path for one array.
        if remaining == 0 {
            return Ok(self.into_iter().map(|item| array![item].into_value()).collect());
        }

        // Fast path for just two arrays.
        if remaining == 1 {
            let Spanned { v: other, span: other_span } =
                args.expect::<Spanned<Array>>("others")?;
            if exact && self.len() != other.len() {
                bail!(
                    other_span,
                    "second array has different length ({}) from first array ({})",
                    other.len(),
                    self.len()
                );
            }
            return Ok(self
                .into_iter()
                .zip(other)
                .map(|(first, second)| array![first, second].into_value())
                .collect());
        }

        // If there is more than one array, we use the manual method.
        let mut out = Self::with_capacity(self.len());
        let arrays = args.all::<Spanned<Array>>()?;
        if exact {
            let errs = arrays
                .iter()
                .filter(|sp| sp.v.len() != self.len())
                .map(|Spanned { v, span }| {
                    SourceDiagnostic::error(
                        *span,
                        eco_format!(
                            "array has different length ({}) from first array ({})",
                            v.len(),
                            self.len()
                        ),
                    )
                })
                .collect::<EcoVec<_>>();
            if !errs.is_empty() {
                return Err(errs);
            }
        }

        let mut iterators =
            arrays.into_iter().map(|i| i.v.into_iter()).collect::<Vec<_>>();

        for this in self {
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
        self,
        /// The engine.
        engine: &mut Engine,
        /// The callsite context.
        context: Tracked<Context>,
        /// The initial value to start with.
        init: Value,
        /// The folding function. Must have two parameters: One for the
        /// accumulated value and one for an item.
        folder: Func,
    ) -> SourceResult<Value> {
        let mut acc = init;
        for item in self {
            acc = folder.call(engine, context, [acc, item])?;
        }
        Ok(acc)
    }

    /// Sums all items (works for all types that can be added).
    #[func]
    pub fn sum(
        self,
        /// What to return if the array is empty. Must be set if the array can
        /// be empty.
        #[named]
        default: Option<Value>,
    ) -> HintedStrResult<Value> {
        let mut iter = self.into_iter();
        let mut acc = iter
            .next()
            .or(default)
            .ok_or("cannot calculate sum of empty array with no default")?;
        for item in iter {
            acc = ops::add(acc, item)?;
        }
        Ok(acc)
    }

    /// Calculates the product all items (works for all types that can be
    /// multiplied).
    #[func]
    pub fn product(
        self,
        /// What to return if the array is empty. Must be set if the array can
        /// be empty.
        #[named]
        default: Option<Value>,
    ) -> HintedStrResult<Value> {
        let mut iter = self.into_iter();
        let mut acc = iter
            .next()
            .or(default)
            .ok_or("cannot calculate product of empty array with no default")?;
        for item in iter {
            acc = ops::mul(acc, item)?;
        }
        Ok(acc)
    }

    /// Whether the given function returns `{true}` for any item in the array.
    #[func]
    pub fn any(
        self,
        /// The engine.
        engine: &mut Engine,
        /// The callsite context.
        context: Tracked<Context>,
        /// The function to apply to each item. Must return a boolean.
        test: Func,
    ) -> SourceResult<bool> {
        for item in self {
            if test.call(engine, context, [item])?.cast::<bool>().at(test.span())? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Whether the given function returns `{true}` for all items in the array.
    #[func]
    pub fn all(
        self,
        /// The engine.
        engine: &mut Engine,
        /// The callsite context.
        context: Tracked<Context>,
        /// The function to apply to each item. Must return a boolean.
        test: Func,
    ) -> SourceResult<bool> {
        for item in self {
            if !test.call(engine, context, [item])?.cast::<bool>().at(test.span())? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Combine all nested arrays into a single flat one.
    #[func]
    pub fn flatten(self) -> Array {
        let mut flat = EcoVec::with_capacity(self.0.len());
        for item in self {
            if let Value::Array(nested) = item {
                flat.extend(nested.flatten());
            } else {
                flat.push(item);
            }
        }
        flat.into()
    }

    /// Return a new array with the same items, but in reverse order.
    #[func(title = "Reverse")]
    pub fn rev(self) -> Array {
        self.into_iter().rev().collect()
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
        self,
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
        for (i, value) in self.into_iter().enumerate() {
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
        self,
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
        let mut iter = self.into_iter();

        if let Some(first) = iter.next() {
            vec.push(first);
        }

        for value in iter {
            vec.push(separator.clone());
            vec.push(value);
        }

        Array(vec)
    }

    /// Splits an array into non-overlapping chunks, starting at the beginning,
    /// ending with a single remainder chunk.
    ///
    /// All chunks but the last have `chunk-size` elements.
    /// If `exact` is set to `{true}`, the remainder is dropped if it
    /// contains less than `chunk-size` elements.
    ///
    /// ```example
    /// #let array = (1, 2, 3, 4, 5, 6, 7, 8)
    /// #array.chunks(3)
    /// #array.chunks(3, exact: true)
    /// ```
    #[func]
    pub fn chunks(
        self,
        /// How many elements each chunk may at most contain.
        chunk_size: NonZeroUsize,
        /// Whether to keep the remainder if its size is less than `chunk-size`.
        #[named]
        #[default(false)]
        exact: bool,
    ) -> Array {
        let to_array = |chunk| Array::from(chunk).into_value();
        if exact {
            self.0.chunks_exact(chunk_size.get()).map(to_array).collect()
        } else {
            self.0.chunks(chunk_size.get()).map(to_array).collect()
        }
    }

    /// Returns sliding windows of `window-size` elements over an array.
    ///
    /// If the array length is less than `window-size`, this will return an empty array.
    ///
    /// ```example
    /// #let array = (1, 2, 3, 4, 5, 6, 7, 8)
    /// #array.windows(5)
    /// ```
    #[func]
    pub fn windows(
        self,
        /// How many elements each window will contain.
        window_size: NonZeroUsize,
    ) -> Array {
        self.0
            .windows(window_size.get())
            .map(|window| Array::from(window).into_value())
            .collect()
    }

    /// Return a sorted version of this array, optionally by a given key
    /// function. The sorting algorithm used is stable.
    ///
    /// Returns an error if two values could not be compared or if the key
    /// function (if given) yields an error.
    #[func]
    pub fn sorted(
        self,
        /// The engine.
        engine: &mut Engine,
        /// The callsite context.
        context: Tracked<Context>,
        /// The callsite span.
        span: Span,
        /// If given, applies this function to the elements in the array to
        /// determine the keys to sort by.
        #[named]
        key: Option<Func>,
    ) -> SourceResult<Array> {
        let mut result = Ok(());
        let mut vec = self.0;
        let mut key_of = |x: Value| match &key {
            // NOTE: We are relying on `comemo`'s memoization of function
            // evaluation to not excessively reevaluate the `key`.
            Some(f) => f.call(engine, context, [x]),
            None => Ok(x),
        };
        vec.make_mut().sort_by(|a, b| {
            // Until we get `try` blocks :)
            match (key_of(a.clone()), key_of(b.clone())) {
                (Ok(a), Ok(b)) => ops::compare(&a, &b).unwrap_or_else(|err| {
                    if result.is_ok() {
                        result = Err(err).at(span);
                    }
                    Ordering::Equal
                }),
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
        self,
        /// The engine.
        engine: &mut Engine,
        /// The callsite context.
        context: Tracked<Context>,
        /// If given, applies this function to the elements in the array to
        /// determine the keys to deduplicate by.
        #[named]
        key: Option<Func>,
    ) -> SourceResult<Array> {
        let mut out = EcoVec::with_capacity(self.0.len());
        let mut key_of = |x: Value| match &key {
            // NOTE: We are relying on `comemo`'s memoization of function
            // evaluation to not excessively reevaluate the `key`.
            Some(f) => f.call(engine, context, [x]),
            None => Ok(x),
        };

        // This algorithm is O(N^2) because we cannot rely on `HashSet` since:
        // 1. We would like to preserve the order of the elements.
        // 2. We cannot hash arbitrary `Value`.
        'outer: for value in self {
            let key = key_of(value.clone())?;
            if out.is_empty() {
                out.push(value);
                continue;
            }

            for second in out.iter() {
                if ops::equal(&key, &key_of(second.clone())?) {
                    continue 'outer;
                }
            }

            out.push(value);
        }

        Ok(Self(out))
    }

    /// Converts an array of pairs into a dictionary.
    /// The first value of each pair is the key, the second the value.
    ///
    /// If the same key occurs multiple times, the last value is selected.
    ///
    /// ```example
    /// #(
    ///   ("apples", 2),
    ///   ("peaches", 3),
    ///   ("apples", 5),
    /// ).to-dict()
    /// ```
    #[func]
    pub fn to_dict(self) -> StrResult<Dict> {
        self.into_iter()
            .map(|value| {
                let value_ty = value.ty();
                let pair = value.cast::<Array>().map_err(|_| {
                    eco_format!("expected (str, any) pairs, found {}", value_ty)
                })?;
                if let [key, value] = pair.as_slice() {
                    let key = key.clone().cast::<Str>().map_err(|_| {
                        eco_format!("expected key of type str, found {}", value.ty())
                    })?;
                    Ok((key, value.clone()))
                } else {
                    bail!("expected pairs of length 2, found length {}", pair.len());
                }
            })
            .collect()
    }

    /// Reduces the elements to a single one, by repeatedly applying a reducing
    /// operation.
    ///
    /// If the array is empty, returns `{none}`, otherwise, returns the result
    /// of the reduction.
    ///
    /// The reducing function is a closure with two arguments: an "accumulator",
    /// and an element.
    ///
    /// For arrays with at least one element, this is the same as [`array.fold`]
    /// with the first element of the array as the initial accumulator value,
    /// folding every subsequent element into it.
    #[func]
    pub fn reduce(
        self,
        /// The engine.
        engine: &mut Engine,
        /// The callsite context.
        context: Tracked<Context>,
        /// The reducing function. Must have two parameters: One for the
        /// accumulated value and one for an item.
        reducer: Func,
    ) -> SourceResult<Value> {
        let mut iter = self.into_iter();
        let mut acc = iter.next().unwrap_or_default();
        for item in iter {
            acc = reducer.call(engine, context, [acc, item])?;
        }
        Ok(acc)
    }
}

/// A value that can be cast to bytes.
pub struct ToArray(Array);

cast! {
    ToArray,
    v: Array => Self(v),
    v: Bytes => Self(v.iter().map(|&b| Value::Int(b.into())).collect()),
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
        repr::pretty_array_like(&pieces, self.len() == 1).into()
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

impl<T: Reflect, const N: usize> Reflect for SmallVec<[T; N]> {
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

impl<T: IntoValue, const N: usize> IntoValue for SmallVec<[T; N]> {
    fn into_value(self) -> Value {
        Value::Array(self.into_iter().map(IntoValue::into_value).collect())
    }
}

impl<T: FromValue> FromValue for Vec<T> {
    fn from_value(value: Value) -> HintedStrResult<Self> {
        value.cast::<Array>()?.into_iter().map(Value::cast).collect()
    }
}

impl<T: FromValue, const N: usize> FromValue for SmallVec<[T; N]> {
    fn from_value(value: Value) -> HintedStrResult<Self> {
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
