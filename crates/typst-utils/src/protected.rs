/// Wraps a type, requiring justification on access.
///
/// On the type-system level, this does not do much, but it makes sure that
/// users of the value think twice and justify their use.
#[derive(Debug, Copy, Clone)]
pub struct Protected<T>(T);

impl<T> Protected<T> {
    /// Wrap a value of type `T`.
    pub fn new(inner: T) -> Self {
        Self(inner)
    }

    /// Rewrap a value extracted via [`into_raw`](Self::into_raw).
    ///
    /// This is distinct from [`new`](Self::new) as it's only meant to be used
    /// for rewrapping and not for initial wrapping.
    pub fn from_raw(inner: T) -> Self {
        Self(inner)
    }

    /// Extract the inner value without justification. The result may only be
    /// used with [`from_raw`](Self::from_raw).
    pub fn into_raw(self) -> T {
        self.0
    }

    /// Access the underlying value, providing justification why it's okay.
    pub fn access(&self, _justification: &'static str) -> &T {
        &self.0
    }
}
