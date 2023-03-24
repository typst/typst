use std::borrow::Cow;
use std::fmt::{self, Debug, Formatter};
use std::ops::Deref;
use std::sync::Arc;

use comemo::Prehashed;

/// A shared buffer that is cheap to clone and hash.
#[derive(Clone, Hash, Eq, PartialEq)]
pub struct Buffer(Arc<Prehashed<Cow<'static, [u8]>>>);

impl Buffer {
    /// Create a buffer from a static byte slice.
    #[must_use]
    pub fn from_static(slice: &'static [u8]) -> Self {
        Self(Arc::new(Prehashed::new(Cow::Borrowed(slice))))
    }

    /// Return a view into the buffer.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        self
    }

    /// Return a copy of the buffer as a vector.
    #[must_use]
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl From<&'static [u8]> for Buffer {
    fn from(bytes: &'static [u8]) -> Self {
        Self::from_static(bytes)
    }
}

impl From<Vec<u8>> for Buffer {
    fn from(vec: Vec<u8>) -> Self {
        Self(Arc::new(Prehashed::new(vec.into())))
    }
}

impl Deref for Buffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<[u8]> for Buffer {
    fn as_ref(&self) -> &[u8] {
        self
    }
}

impl Debug for Buffer {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.pad("Buffer(..)")
    }
}
