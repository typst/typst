use std::fmt::{self, Debug, Formatter};
use std::ops::Deref;
use std::sync::Arc;

use comemo::Prehashed;

/// A shared buffer that is cheap to clone and hash.
#[derive(Clone, Hash, Eq, PartialEq)]
pub struct Buffer(Prehashed<Arc<Vec<u8>>>);

impl Buffer {
    /// Return a view into the buffer.
    pub fn as_slice(&self) -> &[u8] {
        self
    }

    /// Return a copy of the buffer as a vector.
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl From<&[u8]> for Buffer {
    fn from(slice: &[u8]) -> Self {
        Self(Prehashed::new(Arc::new(slice.to_vec())))
    }
}

impl From<Vec<u8>> for Buffer {
    fn from(vec: Vec<u8>) -> Self {
        Self(Prehashed::new(Arc::new(vec)))
    }
}

impl From<Arc<Vec<u8>>> for Buffer {
    fn from(arc: Arc<Vec<u8>>) -> Self {
        Self(Prehashed::new(arc))
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
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("Buffer(..)")
    }
}
