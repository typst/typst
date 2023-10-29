use ecow::EcoString;
use lasso::{Spur, ThreadedRodeo};
use once_cell::sync::Lazy;
use typst_macros::cast;

/// The global string interner.
static INTERNER: Lazy<ThreadedRodeo> = Lazy::new(ThreadedRodeo::new);

/// An interned string.
///
/// The API is purposefully kept small. This is because it might be relatively
/// slow to look up a string in the interner, so we want to avoid doing it
/// unnecessarily. For this reason, the user should use the [`PicoStr::resolve`]
/// method to get the underlying string, such that the lookup is done only once.
#[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct PicoStr(Spur);

impl PicoStr {
    /// Creates a new interned string.
    pub fn new<S: AsRef<str>>(s: S) -> Self {
        Self(INTERNER.get_or_intern(s.as_ref()))
    }

    /// Creates a new interned string from a static string.
    pub fn static_(s: &'static str) -> Self {
        Self(INTERNER.get_or_intern_static(s))
    }

    /// Resolves the interned string.
    pub fn resolve(&self) -> &'static str {
        INTERNER.resolve(&self.0)
    }
}

cast! {
    PicoStr,
    self => self.resolve().into_value(),
    v: EcoString => Self::new(&v),
}

impl From<&'static str> for PicoStr {
    fn from(value: &'static str) -> Self {
        Self::static_(value)
    }
}

impl<'a> From<&'a EcoString> for PicoStr {
    fn from(value: &'a EcoString) -> Self {
        Self::new(value)
    }
}

impl std::fmt::Debug for PicoStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.resolve(), f)
    }
}

impl AsRef<str> for PicoStr {
    fn as_ref(&self) -> &str {
        self.resolve()
    }
}
