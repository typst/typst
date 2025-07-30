//! Definition of the central compilation context.

use std::sync::atomic::{AtomicUsize, Ordering};

use comemo::{Track, Tracked, TrackedMut, Validate};
use ecow::EcoVec;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use rustc_hash::FxHashSet;
use typst_syntax::{FileId, Span};

use crate::World;
use crate::diag::{HintedStrResult, SourceDiagnostic, SourceResult, StrResult, bail};
use crate::foundations::{Styles, Value};
use crate::introspection::Introspector;
use crate::routines::Routines;

/// Holds all data needed during compilation.
pub struct Engine<'a> {
    /// Defines implementation of various Typst compiler routines as a table of
    /// function pointers.
    pub routines: &'a Routines,
    /// The compilation environment.
    pub world: Tracked<'a, dyn World + 'a>,
    /// Provides access to information about the document.
    pub introspector: Tracked<'a, Introspector>,
    /// May hold a span that is currently under inspection.
    pub traced: Tracked<'a, Traced>,
    /// A pure sink for warnings, delayed errors, and spans under inspection.
    pub sink: TrackedMut<'a, Sink>,
    /// The route the engine took during compilation. This is used to detect
    /// cyclic imports and excessive nesting.
    pub route: Route<'a>,
}

impl Engine<'_> {
    /// Handles a result without immediately terminating execution. Instead, it
    /// produces a delayed error that is only promoted to a fatal one if it
    /// remains by the end of the introspection loop.
    pub fn delay<T: Default>(&mut self, result: SourceResult<T>) -> T {
        match result {
            Ok(value) => value,
            Err(errors) => {
                self.sink.delay(errors);
                T::default()
            }
        }
    }

    /// Runs tasks on the engine in parallel.
    pub fn parallelize<P, I, T, U, F>(
        &mut self,
        iter: P,
        f: F,
    ) -> impl Iterator<Item = U> + use<P, I, T, U, F>
    where
        P: IntoIterator<IntoIter = I>,
        I: Iterator<Item = T>,
        T: Send,
        U: Send,
        F: Fn(&mut Engine, T) -> U + Send + Sync,
    {
        let Engine {
            world, introspector, traced, ref route, routines, ..
        } = *self;

        // We collect into a vector and then call `into_par_iter` instead of
        // using `par_bridge` because it does not retain the ordering.
        let work: Vec<T> = iter.into_iter().collect();

        // Work in parallel.
        let mut pairs: Vec<(U, Sink)> = Vec::with_capacity(work.len());
        work.into_par_iter()
            .map(|value| {
                let mut sink = Sink::new();
                let mut engine = Engine {
                    world,
                    introspector,
                    traced,
                    sink: sink.track_mut(),
                    route: route.clone(),
                    routines,
                };
                (f(&mut engine, value), sink)
            })
            .collect_into_vec(&mut pairs);

        // Apply the subsinks to the outer sink.
        for (_, sink) in &mut pairs {
            let sink = std::mem::take(sink);
            self.sink.extend(sink.delayed, sink.warnings, sink.values);
        }

        pairs.into_iter().map(|(output, _)| output)
    }
}

/// May hold a span that is currently under inspection.
#[derive(Default)]
pub struct Traced(Option<Span>);

impl Traced {
    /// Wraps a to-be-traced `Span`.
    ///
    /// Call `Traced::default()` to trace nothing.
    pub fn new(traced: Span) -> Self {
        Self(Some(traced))
    }
}

#[comemo::track]
impl Traced {
    /// Returns the traced span _if_ it is part of the given source file or
    /// `None` otherwise.
    ///
    /// We hide the span if it isn't in the given file so that only results for
    /// the file with the traced span are invalidated.
    pub fn get(&self, id: FileId) -> Option<Span> {
        if self.0.and_then(Span::id) == Some(id) { self.0 } else { None }
    }
}

/// A push-only sink for delayed errors, warnings, and traced values.
///
/// All tracked methods of this type are of the form `(&mut self, ..) -> ()`, so
/// in principle they do not need validation (though that optimization is not
/// yet implemented in comemo).
#[derive(Default, Clone)]
pub struct Sink {
    /// Delayed errors: Those are errors that we can ignore until the last
    /// iteration. For instance, show rules may throw during earlier iterations
    /// because the introspector is not yet ready. We first ignore that and
    /// proceed with empty content and only if the error remains by the end
    /// of the last iteration, we promote it.
    delayed: EcoVec<SourceDiagnostic>,
    /// Warnings emitted during iteration.
    warnings: EcoVec<SourceDiagnostic>,
    /// Hashes of all warning's spans and messages for warning deduplication.
    warnings_set: FxHashSet<u128>,
    /// A sequence of traced values for a span.
    values: EcoVec<(Value, Option<Styles>)>,
}

impl Sink {
    /// The maximum number of traced values.
    pub const MAX_VALUES: usize = 10;

    /// Create a new empty sink.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the stored delayed errors.
    pub fn delayed(&mut self) -> EcoVec<SourceDiagnostic> {
        std::mem::take(&mut self.delayed)
    }

    /// Get the stored warnings.
    pub fn warnings(self) -> EcoVec<SourceDiagnostic> {
        self.warnings
    }

    /// Get the values for the traced span.
    pub fn values(self) -> EcoVec<(Value, Option<Styles>)> {
        self.values
    }

    /// Extend from another sink.
    pub fn extend_from_sink(&mut self, other: Sink) {
        self.extend(other.delayed, other.warnings, other.values);
    }
}

#[comemo::track]
impl Sink {
    /// Push delayed errors.
    pub fn delay(&mut self, errors: EcoVec<SourceDiagnostic>) {
        self.delayed.extend(errors);
    }

    /// Add a warning.
    pub fn warn(&mut self, warning: SourceDiagnostic) {
        // Check if warning is a duplicate.
        let hash = typst_utils::hash128(&(&warning.span, &warning.message));
        if self.warnings_set.insert(hash) {
            self.warnings.push(warning);
        }
    }

    /// Trace a value and optionally styles for the traced span.
    pub fn value(&mut self, value: Value, styles: Option<Styles>) {
        if self.values.len() < Self::MAX_VALUES {
            self.values.push((value, styles));
        }
    }

    /// Extend from parts of another sink.
    fn extend(
        &mut self,
        delayed: EcoVec<SourceDiagnostic>,
        warnings: EcoVec<SourceDiagnostic>,
        values: EcoVec<(Value, Option<Styles>)>,
    ) {
        self.delayed.extend(delayed);
        for warning in warnings {
            self.warn(warning);
        }
        if let Some(remaining) = Self::MAX_VALUES.checked_sub(self.values.len()) {
            self.values.extend(values.into_iter().take(remaining));
        }
    }
}

/// The route the engine took during compilation. This is used to detect
/// cyclic imports and excessive nesting.
pub struct Route<'a> {
    /// The parent route segment, if present.
    ///
    /// This is used when an engine is created from another engine.
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
    /// The upper bound we've established for the parent chain length.
    ///
    /// We don't know the exact length (that would defeat the whole purpose
    /// because it would prevent cache reuse of some computation at different,
    /// non-exceeding depths).
    upper: AtomicUsize,
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

/// The maximum nesting depths. They are different so that even if show rule and
/// call checks are interleaved, for show rule problems we always get the show
/// rule error. The lower the max depth for a kind of error, the higher its
/// precedence compared to the others.
impl Route<'_> {
    /// The maximum stack nesting depth.
    const MAX_SHOW_RULE_DEPTH: usize = 64;

    /// The maximum layout nesting depth.
    const MAX_LAYOUT_DEPTH: usize = 72;

    /// The maximum HTML nesting depth.
    const MAX_HTML_DEPTH: usize = 72;

    /// The maximum function call nesting depth.
    const MAX_CALL_DEPTH: usize = 80;

    /// Ensures that we are within the maximum show rule depth.
    pub fn check_show_depth(&self) -> HintedStrResult<()> {
        if !self.within(Route::MAX_SHOW_RULE_DEPTH) {
            bail!(
                "maximum show rule depth exceeded";
                hint: "maybe a show rule matches its own output";
                hint: "maybe there are too deeply nested elements"
            );
        }
        Ok(())
    }

    /// Ensures that we are within the maximum layout depth.
    pub fn check_layout_depth(&self) -> HintedStrResult<()> {
        if !self.within(Route::MAX_LAYOUT_DEPTH) {
            bail!(
                "maximum layout depth exceeded";
                hint: "try to reduce the amount of nesting in your layout",
            );
        }
        Ok(())
    }

    /// Ensures that we are within the maximum HTML depth.
    pub fn check_html_depth(&self) -> HintedStrResult<()> {
        if !self.within(Route::MAX_HTML_DEPTH) {
            bail!(
                "maximum HTML depth exceeded";
                hint: "try to reduce the amount of nesting of your HTML",
            );
        }
        Ok(())
    }

    /// Ensures that we are within the maximum function call depth.
    pub fn check_call_depth(&self) -> StrResult<()> {
        if !self.within(Route::MAX_CALL_DEPTH) {
            bail!("maximum function call depth exceeded");
        }
        Ok(())
    }
}

#[comemo::track]
#[allow(clippy::needless_lifetimes)]
impl<'a> Route<'a> {
    /// Whether the given id is part of the route.
    pub fn contains(&self, id: FileId) -> bool {
        self.id == Some(id) || self.outer.is_some_and(|outer| outer.contains(id))
    }

    /// Whether the route's depth is less than or equal to the given depth.
    pub fn within(&self, depth: usize) -> bool {
        // We only need atomicity and no synchronization of other operations, so
        // `Relaxed` is fine.
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
            upper: AtomicUsize::new(self.upper.load(Ordering::Relaxed)),
        }
    }
}
