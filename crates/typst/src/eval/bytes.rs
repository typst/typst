use std::borrow::Cow;
use std::fmt::{self, Debug, Formatter};
use std::ops::Deref;
use std::sync::Arc;

use comemo::Prehashed;
use ecow::{eco_format, EcoString};
use serde::{Serialize, Serializer};

use crate::diag::StrResult;

use super::Value;

/// A shared byte buffer that is cheap to clone and hash.
#[derive(Clone, Hash, Eq, PartialEq)]
pub struct Bytes(Arc<Prehashed<Cow<'static, [u8]>>>);

impl Bytes {
    /// Create a buffer from a static byte slice.
    pub fn from_static(slice: &'static [u8]) -> Self {
        Self(Arc::new(Prehashed::new(Cow::Borrowed(slice))))
    }

    /// Get the byte at the given index.
    pub fn at(&self, index: i64, default: Option<Value>) -> StrResult<Value> {
        self.locate_opt(index)
            .and_then(|i| self.0.get(i).map(|&b| Value::Int(b as i64)))
            .or(default)
            .ok_or_else(|| out_of_bounds_no_default(index, self.len()))
    }

    /// Extract a contiguous subregion of the bytes.
    pub fn slice(&self, start: i64, end: Option<i64>) -> StrResult<Self> {
        let start = self.locate(start)?;
        let end = self.locate(end.unwrap_or(self.len() as i64))?.max(start);
        Ok(self.0[start..end].into())
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

impl From<&[u8]> for Bytes {
    fn from(slice: &[u8]) -> Self {
        Self(Arc::new(Prehashed::new(slice.to_vec().into())))
    }
}

impl From<Vec<u8>> for Bytes {
    fn from(vec: Vec<u8>) -> Self {
        Self(Arc::new(Prehashed::new(vec.into())))
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

impl Debug for Bytes {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "bytes({})", self.len())
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
