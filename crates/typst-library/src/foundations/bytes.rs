use std::any::Any;
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::{Add, AddAssign, Deref};
use std::str::Utf8Error;
use std::sync::Arc;

use ecow::{EcoString, eco_format};
use serde::{Serialize, Serializer};
use typst_syntax::{Lines, Source};
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

    /// Attempts to take ownership of an underlying vector. If this is not
    /// possible, returns a newly allocated vector with the byte data.
    ///
    /// For the underlying allocation to be reused, the bytes must have been
    /// created via [`Bytes::new`] from a [`Vec<u8>`] and the reference count
    /// must be 1.
    pub fn into_vec(mut self) -> Vec<u8> {
        match self.to_underlying_mut::<Vec<u8>>() {
            Some(vec) => std::mem::take(vec),
            None => self.as_slice().to_vec(),
        }
    }

    /// Attempts to take ownership of an underlying string or byte vector. If
    /// this is not possible, returns a newly allocated vector with the byte
    /// data.
    ///
    /// For the underlying allocation to be reused, the bytes must have been
    /// created via [`Bytes::new`] from a [`Vec<u8>`] or via
    /// [`Bytes::from_string`] from a [`String`] and the reference count must be
    /// 1.
    pub fn into_string(mut self) -> Result<String, IntoStringError> {
        if let Some(string) = self.to_underlying_string_mut::<String>() {
            return Ok(std::mem::take(string));
        }

        let result = if let Some(vec) = self.to_underlying_mut::<Vec<u8>>() {
            match String::from_utf8(std::mem::take(vec)) {
                Ok(string) => return Ok(string),
                Err(err) => {
                    let error = err.utf8_error();
                    *vec = err.into_bytes();
                    Err(error)
                }
            }
        } else {
            self.as_str().map(ToOwned::to_owned)
        };

        result.map_err(|error| IntoStringError { bytes: self, error })
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

    /// Try to produce line metadata for these bytes. Fails if the bytes are not
    /// UTF-8 decodable.
    ///
    /// If the bytes were created from a [`Source`] file via
    /// [`Bytes::from_string`], the source file's line metadata is reused.
    /// Otherwise, line metadata is computed with internal memoization.
    pub fn lines(&self) -> Result<Lines<String>, Utf8Error> {
        #[comemo::memoize]
        fn compute(bytes: &Bytes) -> Result<Lines<String>, Utf8Error> {
            let text = bytes.as_str()?;
            Ok(Lines::new(text.to_string()))
        }

        // Small optimization: If this comes from a source file via
        // `Bytes::from_string`, we can directly use its lines.
        match self.to_underlying_string::<Source>() {
            Some(source) => Ok(source.lines().clone()),
            None => compute(self),
        }
    }
}

impl Bytes {
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

    /// Try to access a vector this was built from via [`Bytes::new`].
    fn to_underlying_mut<T>(&mut self) -> Option<&mut T>
    where
        T: AsRef<[u8]> + Send + Sync + 'static,
    {
        Arc::get_mut(&mut self.0).and_then(|unique| {
            let inner: &mut dyn Bytelike = &mut **unique;
            (inner as &mut dyn Any).downcast_mut::<T>()
        })
    }

    /// Try to access a string this was built from via [`Bytes::from_string`].
    fn to_underlying_string<T>(&self) -> Option<&T>
    where
        T: AsRef<str> + Send + Sync + 'static,
    {
        (self.inner() as &dyn Any)
            .downcast_ref::<StrWrapper<T>>()
            .map(|wrapper| &wrapper.0)
    }

    /// Try to mutably access a string this was built from via [`Bytes::from_string`].
    fn to_underlying_string_mut<T>(&mut self) -> Option<&mut T>
    where
        T: AsRef<str> + Send + Sync + 'static,
    {
        Arc::get_mut(&mut self.0).and_then(|unique| {
            let inner: &mut dyn Bytelike = &mut **unique;
            (inner as &mut dyn Any)
                .downcast_mut::<StrWrapper<T>>()
                .map(|wrapper| &mut wrapper.0)
        })
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
        } else if let Some(vec) = self.to_underlying_mut::<Vec<u8>>() {
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

/// An error that can occur in [`Bytes::into_string`].
#[derive(Debug)]
pub struct IntoStringError {
    pub bytes: Bytes,
    pub error: Utf8Error,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Round-tripping with lone ownership should retain the same string.
    #[test]
    fn test_bytes_into_string_lone() {
        let s1 = String::from("hello world");
        let p1 = s1.as_ptr();
        let s2 = Bytes::from_string(s1).into_string().unwrap();
        let p2 = s2.as_ptr();
        assert!(std::ptr::eq(p1, p2));
    }

    /// Round-tripping with shared ownership can yield a copy.
    #[test]
    fn test_bytes_into_string_shared() {
        let s1 = String::from("hello world");
        let p1 = s1.as_ptr();
        let x = Bytes::from_string(s1);
        let y = x.clone();
        let s2 = x.into_string().unwrap();
        let p2 = s2.as_ptr();
        let s3 = y.into_string().unwrap();
        let p3 = s3.as_ptr();
        // The first one yields a copy.
        assert!(!std::ptr::eq(p1, p2));
        // The last one yields the original string.
        assert!(std::ptr::eq(p1, p3));
    }

    /// Vector can also be reused as string.
    #[test]
    fn test_bytes_into_string_from_vec() {
        let v1 = String::from("hello world").into_bytes();
        let p1 = v1.as_ptr();
        let v2 = Bytes::new(v1).into_string().unwrap().into_bytes();
        let p2 = v2.as_ptr();
        assert!(std::ptr::eq(p1, p2));
    }

    /// UTF-8 error should retain the original bytes if it's a vector that could
    /// become a string.
    #[test]
    fn test_bytes_into_string_from_vec_error() {
        let s = b"hello world\xFF";
        let err = Bytes::new(Vec::from(s)).into_string().unwrap_err();
        assert_eq!(err.bytes.as_slice(), s);
    }
}
