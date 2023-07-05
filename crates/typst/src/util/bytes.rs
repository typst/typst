use std::borrow::Cow;
use std::fmt::{self, Debug, Formatter};
use std::ops::Deref;
use std::sync::Arc;

use comemo::Prehashed;

/// A shared byte buffer that is cheap to clone and hash.
#[derive(Clone, Hash, Eq, PartialEq)]
pub struct Bytes(Arc<Prehashed<Cow<'static, [u8]>>>);

impl Bytes {
    /// Create a buffer from a static byte slice.
    pub fn from_static(slice: &'static [u8]) -> Self {
        Self(Arc::new(Prehashed::new(Cow::Borrowed(slice))))
    }

    /// Return a view into the buffer.
    pub fn as_slice(&self) -> &[u8] {
        self
    }

    /// Return a copy of the buffer as a vector.
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
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
