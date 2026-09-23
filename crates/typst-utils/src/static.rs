use std::hash::{Hash, Hasher};
use std::ops::Deref;

/// A container around a static reference that is cheap to clone and hash.
#[derive(Debug)]
pub struct Static<T: 'static>(pub &'static T);

impl<T> Deref for Static<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<T> Copy for Static<T> {}

impl<T> Clone for Static<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Eq for Static<T> {}

impl<T> PartialEq for Static<T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

impl<T> Hash for Static<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.0 as *const T as usize);
    }
}
