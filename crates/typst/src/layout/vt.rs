use comemo::{Tracked, TrackedMut};

use crate::diag::SourceResult;
use crate::eval::Tracer;
use crate::introspection::{Introspector, Locator};
use crate::World;

/// A virtual typesetter.
///
/// Holds the state needed during compilation.
pub struct Vt<'a> {
    /// The compilation environment.
    pub world: Tracked<'a, dyn World + 'a>,
    /// Provides access to information about the document.
    pub introspector: Tracked<'a, Introspector>,
    /// Provides stable identities to elements.
    pub locator: &'a mut Locator<'a>,
    /// The tracer for inspection of the values an expression produces.
    pub tracer: TrackedMut<'a, Tracer>,
}

impl Vt<'_> {
    /// Perform a fallible operation that does not immediately terminate further
    /// execution. Instead it produces a delayed error that is only promoted to
    /// a fatal one if it remains at the end of the introspection loop.
    pub fn delayed<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce(&mut Self) -> SourceResult<T>,
        T: Default,
    {
        match f(self) {
            Ok(value) => value,
            Err(errors) => {
                self.tracer.delay(errors);
                T::default()
            }
        }
    }
}
