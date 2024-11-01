use std::borrow::Cow;
use std::fmt::{self, Debug, Formatter};
use std::ops::{Add, AddAssign, Deref};
use std::sync::Arc;

use ecow::{eco_format, EcoString};
use serde::{Serialize, Serializer};
use typst_utils::LazyHash;

use crate::diag::{bail, StrResult};
use crate::foundations::{cast, func, scope, ty, Array, Reflect, Repr, Str, Value};

/// A sequence of bytes.
///
/// This is conceptually similar to an array of [integers]($int) between `{0}`
/// and `{255}`, but represented much more efficiently. You can iterate over it
/// using a [for loop]($scripting/#loops).
///
/// You can convert
/// - a [string]($str) or an [array] of integers to bytes with the [`bytes`]
///   constructor
/// - bytes to a string with the [`str`] constructor, with UTF-8 encoding
/// - bytes to an array of integers with the [`array`] constructor
///
/// When [reading]($read) data from a file, you can decide whether to load it
/// as a string or as raw bytes.
///
/// ```example
/// #bytes((123, 160, 22, 0)) \
/// #bytes("Hello 😃")
///
/// #let data = read(
///   "rhino.png",
///   encoding: none,
/// )
///
/// // Magic bytes.
/// #array(data.slice(0, 4)) \
/// #str(data.slice(1, 4))
/// ```
#[ty(scope, cast)]
#[derive(Clone, Hash, Eq, PartialEq)]
pub struct Bytes(Arc<LazyHash<Cow<'static, [u8]>>>);

impl Bytes {
    /// Create a buffer from a static byte slice.
    pub fn from_static(slice: &'static [u8]) -> Self {
        Self(Arc::new(LazyHash::new(Cow::Borrowed(slice))))
    }

    /// Return `true` if the length is 0.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Return a view into the buffer.
    pub fn as_slice(&self) -> &[u8] {
        self
    }

    /// Return a copy of the buffer as a vector.
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    /// Resolve an index or throw an out of bounds error.
    fn locate(&self, index: i64) -> StrResult<usize> {
        self.locate_opt(index).ok_or_else(|| out_of_bounds(index, self.len()))
    }

    /// Resolve an index, if it is within bounds.
    ///
    /// `index == len` is considered in bounds.
    fn locate_opt(&self, index: i64) -> Option<usize> {
        let wrapped =
            if index >= 0 { Some(index) } else { (self.len() as i64).checked_add(index) };

        wrapped
            .and_then(|v| usize::try_from(v).ok())
            .filter(|&v| v <= self.0.len())
    }
}

#[scope]
impl Bytes {
    /// Converts a value to bytes.
    ///
    /// - Strings are encoded in UTF-8.
    /// - Arrays of integers between `{0}` and `{255}` are converted directly. The
    ///   dedicated byte representation is much more efficient than the array
    ///   representation and thus typically used for large byte buffers (e.g. image
    ///   data).
    ///
    /// ```example
    /// #bytes("Hello 😃") \
    /// #bytes((123, 160, 22, 0))
    /// ```
    #[func(constructor)]
    pub fn construct(
        /// The value that should be converted to bytes.
        value: ToBytes,
    ) -> Bytes {
        value.0
    }

    /// The length in bytes.
    #[func(title = "Length")]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns the byte at the specified index. Returns the default value if
    /// the index is out of bounds or fails with an error if no default value
    /// was specified.
    #[func]
    pub fn at(
        &self,
        /// The index at which to retrieve the byte.
        index: i64,
        /// A default value to return if the index is out of bounds.
        #[named]
        default: Option<Value>,
    ) -> StrResult<Value> {
        self.locate_opt(index)
            .and_then(|i| self.0.get(i).map(|&b| Value::Int(b.into())))
            .or(default)
            .ok_or_else(|| out_of_bounds_no_default(index, self.len()))
    }

    /// Extracts a subslice of the bytes. Fails with an error if the start or end
    /// index is out of bounds.
    #[func]
    pub fn slice(
        &self,
        /// The start index (inclusive).
        start: i64,
        /// The end index (exclusive). If omitted, the whole slice until the end
        /// is extracted.
        #[default]
        end: Option<i64>,
        /// The number of items to extract. This is equivalent to passing
        /// `start + count` as the `end` position. Mutually exclusive with
        /// `end`.
        #[named]
        count: Option<i64>,
    ) -> StrResult<Bytes> {
        let mut end = end;
        if end.is_none() {
            end = count.map(|c: i64| start + c);
        }
        let start = self.locate(start)?;
        let end = self.locate(end.unwrap_or(self.len() as i64))?.max(start);
        Ok(self.0[start..end].into())
    }
}

impl Debug for Bytes {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Bytes({})", self.len())
    }
}

impl Repr for Bytes {
    fn repr(&self) -> EcoString {
        eco_format!("bytes({})", self.len())
    }
}

impl Deref for Bytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<[u8]> for Bytes {
    fn as_ref(&self) -> &[u8] {
        self
    }
}

impl From<&[u8]> for Bytes {
    fn from(slice: &[u8]) -> Self {
        Self(Arc::new(LazyHash::new(slice.to_vec().into())))
    }
}

impl From<Vec<u8>> for Bytes {
    fn from(vec: Vec<u8>) -> Self {
        Self(Arc::new(LazyHash::new(vec.into())))
    }
}

impl Add for Bytes {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self += rhs;
        self
    }
}

impl AddAssign for Bytes {
    fn add_assign(&mut self, rhs: Self) {
        if rhs.is_empty() {
            // Nothing to do
        } else if self.is_empty() {
            *self = rhs;
        } else if Arc::strong_count(&self.0) == 1 && matches!(**self.0, Cow::Owned(_)) {
            Arc::make_mut(&mut self.0).to_mut().extend_from_slice(&rhs);
        } else {
            *self = Self::from([self.as_slice(), rhs.as_slice()].concat());
        }
    }
}

impl Serialize for Bytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&eco_format!("{self:?}"))
        } else {
            serializer.serialize_bytes(self)
        }
    }
}

/// A value that can be cast to bytes.
pub struct ToBytes(Bytes);

cast! {
    ToBytes,
    v: Str => Self(v.as_bytes().into()),
    v: Array => Self(v.iter()
        .map(|item| match item {
            Value::Int(byte @ 0..=255) => Ok(*byte as u8),
            Value::Int(_) => bail!("number must be between 0 and 255"),
            value => Err(<u8 as Reflect>::error(value)),
        })
        .collect::<Result<Vec<u8>, _>>()?
        .into()
    ),
    v: Bytes => Self(v),
}

/// The out of bounds access error message.
#[cold]
fn out_of_bounds(index: i64, len: usize) -> EcoString {
    eco_format!("byte index out of bounds (index: {index}, len: {len})")
}

/// The out of bounds access error message when no default value was given.
#[cold]
fn out_of_bounds_no_default(index: i64, len: usize) -> EcoString {
    eco_format!(
        "byte index out of bounds (index: {index}, len: {len}) \
         and no default value was specified",
    )
}
