use std::any::Any;
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::{Add, AddAssign, Deref};
use std::str::Utf8Error;
use std::sync::Arc;

use ecow::{EcoString, eco_format};
use serde::{Serialize, Serializer};
use typst_syntax::Lines;
use typst_utils::LazyHash;

use crate::diag::{StrResult, bail};
use crate::foundations::{Array, Reflect, Repr, Str, Value, cast, func, scope, ty};

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
#[derive(Clone, Hash)]
pub struct Bytes(Arc<LazyHash<dyn Bytelike>>);

impl Bytes {
    /// Create `Bytes` from anything byte-like.
    ///
    /// The `data` type will directly back this bytes object. This means you can
    /// e.g. pass `&'static [u8]` or `[u8; 8]` and no extra vector will be
    /// allocated.
    ///
    /// If the type is `Vec<u8>` and the `Bytes` are unique (i.e. not cloned),
    /// the vector will be reused when mutating to the `Bytes`.
    ///
    /// If your source type is a string, prefer [`Bytes::from_string`] to
    /// directly use the UTF-8 encoded string data without any copying.
    pub fn new<T>(data: T) -> Self
    where
        T: AsRef<[u8]> + Send + Sync + 'static,
    {
        Self(Arc::new(LazyHash::new(data)))
    }

    /// Create `Bytes` from anything string-like, implicitly viewing the UTF-8
    /// representation.
    ///
    /// The `data` type will directly back this bytes object. This means you can
    /// e.g. pass `String` or `EcoString` without any copying.
    pub fn from_string<T>(data: T) -> Self
    where
        T: AsRef<str> + Send + Sync + 'static,
    {
        Self(Arc::new(LazyHash::new(StrWrapper(data))))
    }

    /// Return `true` if the length is 0.
    pub fn is_empty(&self) -> bool {
        self.as_slice().is_empty()
    }

    /// Return a view into the bytes.
    pub fn as_slice(&self) -> &[u8] {
        self
    }

    /// Try to view the bytes as an UTF-8 string.
    ///
    /// If these bytes were created via `Bytes::from_string`, UTF-8 validation
    /// is skipped.
    pub fn as_str(&self) -> Result<&str, Utf8Error> {
        self.inner().as_str()
    }

    /// Return a copy of the bytes as a vector.
    pub fn to_vec(&self) -> Vec<u8> {
        self.as_slice().to_vec()
    }

    /// Try to turn the bytes into a `Str`.
    ///
    /// - If these bytes were created via `Bytes::from_string::<Str>`, the
    ///   string is cloned directly.
    /// - If these bytes were created via `Bytes::from_string`, but from a
    ///   different type of string, UTF-8 validation is still skipped.
    pub fn to_str(&self) -> Result<Str, Utf8Error> {
        match (self.inner() as &dyn Any).downcast_ref::<Str>() {
            Some(string) => Ok(string.clone()),
            None => self.as_str().map(Into::into),
        }
    }

    /// Resolve an index or throw an out of bounds error.
    fn locate(&self, index: i64) -> StrResult<usize> {
        self.locate_opt(index).ok_or_else(|| out_of_bounds(index, self.len()))
    }

    /// Resolve an index, if it is within bounds.
    ///
    /// `index == len` is considered in bounds.
    fn locate_opt(&self, index: i64) -> Option<usize> {
        let len = self.as_slice().len();
        let wrapped =
            if index >= 0 { Some(index) } else { (len as i64).checked_add(index) };
        wrapped.and_then(|v| usize::try_from(v).ok()).filter(|&v| v <= len)
    }

    /// Access the inner `dyn Bytelike`.
    fn inner(&self) -> &dyn Bytelike {
        &**self.0
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
        self.as_slice().len()
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
            .and_then(|i| self.as_slice().get(i).map(|&b| Value::Int(b.into())))
            .or(default)
            .ok_or_else(|| out_of_bounds_no_default(index, self.len()))
    }

    /// Extracts a subslice of the bytes. Fails with an error if the start or
    /// end index is out of bounds.
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
        let start = self.locate(start)?;
        let end = end.or(count.map(|c| start as i64 + c));
        let end = self.locate(end.unwrap_or(self.len() as i64))?.max(start);
        let slice = &self.as_slice()[start..end];

        // We could hold a view into the original bytes here instead of
        // making a copy, but it's unclear when that's worth it. Java
        // originally did that for strings, but went back on it because a
        // very small view into a very large buffer would be a sort of
        // memory leak.
        Ok(Bytes::new(slice.to_vec()))
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
        self.inner().as_bytes()
    }
}

impl Eq for Bytes {}

impl PartialEq for Bytes {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl AsRef<[u8]> for Bytes {
    fn as_ref(&self) -> &[u8] {
        self
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
        } else if let Some(vec) = Arc::get_mut(&mut self.0).and_then(|unique| {
            let inner: &mut dyn Bytelike = &mut **unique;
            (inner as &mut dyn Any).downcast_mut::<Vec<u8>>()
        }) {
            vec.extend_from_slice(&rhs);
        } else {
            *self = Self::new([self.as_slice(), rhs.as_slice()].concat());
        }
    }
}

impl Serialize for Bytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.repr())
        } else {
            serializer.serialize_bytes(self)
        }
    }
}

impl TryFrom<&Bytes> for Lines<String> {
    type Error = Utf8Error;

    #[comemo::memoize]
    fn try_from(value: &Bytes) -> Result<Lines<String>, Utf8Error> {
        let text = value.as_str()?;
        Ok(Lines::new(text.to_string()))
    }
}

/// Any type that can back a byte buffer.
trait Bytelike: Any + Send + Sync {
    fn as_bytes(&self) -> &[u8];
    fn as_str(&self) -> Result<&str, Utf8Error>;
}

impl<T> Bytelike for T
where
    T: AsRef<[u8]> + Send + Sync + 'static,
{
    fn as_bytes(&self) -> &[u8] {
        self.as_ref()
    }

    fn as_str(&self) -> Result<&str, Utf8Error> {
        std::str::from_utf8(self.as_ref())
    }
}

impl Hash for dyn Bytelike {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_bytes().hash(state);
    }
}

/// Makes string-like objects usable with `Bytes`.
struct StrWrapper<T>(T);

impl<T> Bytelike for StrWrapper<T>
where
    T: AsRef<str> + Send + Sync + 'static,
{
    fn as_bytes(&self) -> &[u8] {
        self.0.as_ref().as_bytes()
    }

    fn as_str(&self) -> Result<&str, Utf8Error> {
        Ok(self.0.as_ref())
    }
}

/// A value that can be cast to bytes.
pub struct ToBytes(Bytes);

cast! {
    ToBytes,
    v: Str => Self(Bytes::from_string(v)),
    v: Array => Self(v.iter()
        .map(|item| match item {
            Value::Int(byte @ 0..=255) => Ok(*byte as u8),
            Value::Int(_) => bail!("number must be between 0 and 255"),
            value => Err(<u8 as Reflect>::error(value)),
        })
        .collect::<Result<Vec<u8>, _>>()
        .map(Bytes::new)?
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
