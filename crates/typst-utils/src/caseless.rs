use std::borrow::Borrow;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::ops::Deref;

use bytemuck::TransparentWrapper;

/// Wraps a string and makes equality and hashing case-insensitive.
///
/// Disclaimer: Currently, this internally uses [`char::to_lowercase`]. We
/// should use `char::to_casefold_unnormalized` once it is stable. But for
/// now, this is good enough. Currently, this is just used for font family
/// comparisons and those previously used `str::to_lowercase`.
#[derive(Debug, Copy, Clone, TransparentWrapper)]
#[repr(transparent)]
pub struct Caseless<S: ?Sized>(pub S);

impl<S: ?Sized> Caseless<S> {
    /// Turns a reference to a string into a reference to a [`Caseless`].
    pub fn wrap(string: &S) -> &Self {
        TransparentWrapper::wrap_ref(string)
    }
}

impl<S: ?Sized + Borrow<str>> Caseless<S> {
    /// Returns the case-folded chars that make up the string.
    fn folded(&self) -> impl Iterator<Item = char> {
        self.0.borrow().chars().flat_map(char::to_lowercase)
    }
}

impl<S: ?Sized + Deref> Deref for Caseless<S> {
    type Target = Caseless<S::Target>;

    fn deref(&self) -> &Self::Target {
        Caseless::wrap(&self.0)
    }
}

impl<S: ?Sized + Borrow<str>> Eq for Caseless<S> {}

impl<S: ?Sized + Borrow<str>> PartialEq for Caseless<S> {
    fn eq(&self, other: &Self) -> bool {
        self.folded().eq(other.folded())
    }
}

impl<S: ?Sized + Borrow<str>> Ord for Caseless<S> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.folded().cmp(other.folded())
    }
}

impl<S: ?Sized + Borrow<str>> PartialOrd for Caseless<S> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<S: ?Sized + Borrow<str>> Hash for Caseless<S> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for c in self.folded() {
            state.write(c.encode_utf8(&mut [0; 4]).as_bytes());
        }
        // See [`str::hash`].
        state.write_u8(0xFF);
    }
}

impl<S> Borrow<Caseless<str>> for Caseless<S>
where
    S: Borrow<str>,
{
    fn borrow(&self) -> &Caseless<str> {
        Caseless::wrap(self.0.borrow())
    }
}
