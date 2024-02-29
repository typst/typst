//! Definition of the central compilation context.

use std::sync::atomic::{AtomicUsize, Ordering};

use comemo::{Track, Tracked, TrackedMut, Validate};

use crate::diag::SourceResult;
use crate::eval::Tracer;
use crate::introspection::{Introspector, Locator};
use crate::syntax::FileId;
use crate::World;

/// Holds all data needed during compilation.
pub struct Engine<'a> {
    /// The compilation environment.
    pub world: Tracked<'a, dyn World + 'a>,
    /// Provides access to information about the document.
    pub introspector: Tracked<'a, Introspector>,
    /// The route the engine took during compilation. This is used to detect
    /// cyclic imports and too much nesting.
    pub route: Route<'a>,
    /// Provides stable identities to elements.
    pub locator: &'a mut Locator<'a>,
    /// The tracer for inspection of the values an expression produces.
    pub tracer: TrackedMut<'a, Tracer>,
}

impl Engine<'_> {
    /// Performs a fallible operation that does not immediately terminate further
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

/// The route the engine took during compilation. This is used to detect
/// cyclic imports and too much nesting.
pub struct Route<'a> {
    // We need to override the constraint's lifetime here so that `Tracked` is
    // covariant over the constraint. If it becomes invariant, we're in for a
    // world of lifetime pain.
    outer: Option<Tracked<'a, Self, <Route<'static> as Validate>::Constraint>>,
    /// This is set if this route segment was inserted through the start of a
    /// module evaluation.
    id: Option<FileId>,
    /// This is set whenever we enter a function, nested layout, or are applying
    /// a show rule. The length of this segment plus the lengths of all `outer`
    /// route segments make up the length of the route. If the length of the
    /// route exceeds `MAX_DEPTH`, then we throw a "maximum ... depth exceeded"
    /// error.
    len: usize,
    /// The upper bound we've established for the parent chain length. We don't
    /// know the exact length (that would defeat the whole purpose because it
    /// would prevent cache reuse of some computation at different,
    /// non-exceeding depths).
    upper: AtomicUsize,
}

/// The maximum nesting depths. They are different so that even if show rule and
/// call checks are interleaved, show rule problems we always get the show rule.
/// The lower the max depth for a kind of error, the higher its precedence
/// compared to the others.
impl Route<'_> {
    /// The maximum stack nesting depth.
    pub const MAX_SHOW_RULE_DEPTH: usize = 64;

    /// The maximum layout nesting depth.
    pub const MAX_LAYOUT_DEPTH: usize = 72;

    /// The maximum function call nesting depth.
    pub const MAX_CALL_DEPTH: usize = 80;
}

impl<'a> Route<'a> {
    /// Create a new, empty route.
    pub fn root() -> Self {
        Self {
            id: None,
            outer: None,
            len: 0,
            upper: AtomicUsize::new(0),
        }
    }

    /// Extend the route with another segment with a default length of 1.
    pub fn extend(outer: Tracked<'a, Self>) -> Self {
        Route {
            outer: Some(outer),
            id: None,
            len: 1,
            upper: AtomicUsize::new(usize::MAX),
        }
    }

    /// Attach a file id to the route segment.
    pub fn with_id(self, id: FileId) -> Self {
        Self { id: Some(id), ..self }
    }

    /// Set the length of the route segment to zero.
    pub fn unnested(self) -> Self {
        Self { len: 0, ..self }
    }

    /// Start tracking this route.
    ///
    /// In comparison to [`Track::track`], this method skips this chain link
    /// if it does not contribute anything.
    pub fn track(&self) -> Tracked<'_, Self> {
        match self.outer {
            Some(outer) if self.id.is_none() && self.len == 0 => outer,
            _ => Track::track(self),
        }
    }

    /// Increase the nesting depth for this route segment.
    pub fn increase(&mut self) {
        self.len += 1;
    }

    /// Decrease the nesting depth for this route segment.
    pub fn decrease(&mut self) {
        self.len -= 1;
    }
}

#[comemo::track]
impl<'a> Route<'a> {
    /// Whether the given id is part of the route.
    pub fn contains(&self, id: FileId) -> bool {
        self.id == Some(id) || self.outer.is_some_and(|outer| outer.contains(id))
    }

    /// Whether the route's depth is less than or equal to the given depth.
    pub fn within(&self, depth: usize) -> bool {
        use Ordering::Relaxed;

        let upper = self.upper.load(Relaxed);
        if upper.saturating_add(self.len) <= depth {
            return true;
        }

        match self.outer {
            Some(_) if depth < self.len => false,
            Some(outer) => {
                let within = outer.within(depth - self.len);
                if within && depth < upper {
                    // We don't want to accidentally increase the upper bound,
                    // hence the compare-exchange.
                    self.upper.compare_exchange(upper, depth, Relaxed, Relaxed).ok();
                }
                within
            }
            None => true,
        }
    }
}

impl Default for Route<'_> {
    fn default() -> Self {
        Self::root()
    }
}

impl Clone for Route<'_> {
    fn clone(&self) -> Self {
        Self {
            outer: self.outer,
            id: self.id,
            len: self.len,
            // The ordering doesn't really matter since it's the upper bound
            // is only an optimization.
            upper: AtomicUsize::new(self.upper.load(Ordering::Relaxed)),
        }
    }
}
