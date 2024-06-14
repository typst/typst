//! Definition of the central compilation context.

use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};

use comemo::{Track, Tracked, TrackedMut, Validate};
use ecow::EcoVec;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};

use crate::diag::{self, Severity, SourceDiagnostic, SourceResult, Trace, Tracepoint};
use crate::foundations::{Styles, Value};
use crate::introspection::Introspector;
use crate::syntax::{ast, FileId, Span};
use crate::World;

/// Holds all data needed during compilation.
pub struct Engine<'a> {
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
    /// Performs a fallible operation that does not immediately terminate further
    /// execution. Instead it produces a delayed error that is only promoted to
    /// a fatal one if it remains at the end of the introspection loop.
    pub fn delay<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce(&mut Self) -> SourceResult<T>,
        T: Default,
    {
        match f(self) {
            Ok(value) => value,
            Err(errors) => {
                self.sink.delay(errors);
                T::default()
            }
        }
    }

    /// Runs tasks on the engine in parallel.
    pub fn parallelize<P, I, T, U, F>(&mut self, iter: P, f: F) -> impl Iterator<Item = U>
    where
        P: IntoIterator<IntoIter = I>,
        I: Iterator<Item = T>,
        T: Send,
        U: Send,
        F: Fn(&mut Engine, T) -> U + Send + Sync,
    {
        let Engine { world, introspector, traced, ref route, .. } = *self;

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
                };
                (f(&mut engine, value), sink)
            })
            .collect_into_vec(&mut pairs);

        // Apply the subsinks to the outer sink.
        for (_, sink) in &mut pairs {
            Sink::extend_tracked(&mut self.sink, std::mem::take(sink));
        }

        pairs.into_iter().map(|(output, _)| output)
    }

    /// Executes some code with a tracepoint for diagnostics.
    /// The tracepoint is added to any diagnostics returned by the function,
    /// as well as any warnings emitted by it.
    pub fn tracepoint<F, M, T>(
        &mut self,
        make_point: M,
        span: Span,
        f: F,
    ) -> SourceResult<T>
    where
        F: FnOnce(&mut Engine) -> SourceResult<T>,
        M: Copy + Fn() -> Tracepoint,
    {
        let Engine { world, introspector, traced, ref route, .. } = *self;

        // Create a temporary sink to accumulate all warnings produced by
        // this call (either directly or due to a nested call). Later, we
        // add a tracepoint to those warnings, indicating this call was
        // part of the call stack that led to the warning being raised,
        // thus allowing suppression of the warning through this call.
        let mut sink = Sink::new();
        let mut engine = Engine {
            world,
            introspector,
            traced,
            sink: sink.track_mut(),
            route: route.clone(),
        };

        // Trace errors immediately, followed by warnings on the sink.
        let call_result = f(&mut engine).trace(world, make_point, span);
        sink.trace_warnings(world, make_point, span);

        // Push the accumulated warnings and other fields back to the
        // original sink after we have modified them. This is needed so the
        // warnings are properly returned by compilation later.
        Sink::extend_tracked(&mut self.sink, sink);

        call_result
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
        if self.0.and_then(Span::id) == Some(id) {
            self.0
        } else {
            None
        }
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
    warnings_set: HashSet<u128>,
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

    /// Extend the destination sink with the data from the source sink.
    /// This calls a tracked function on the destination unless the source
    /// is fully empty (which is usually the case).
    pub fn extend_tracked(destination: &mut TrackedMut<'_, Self>, source: Sink) {
        let Sink { delayed, warnings, values, .. } = source;
        if !delayed.is_empty() || !warnings.is_empty() || !values.is_empty() {
            destination.extend(delayed, warnings, values);
        }
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

    /// Adds a tracepoint to all warnings outside the given span.
    pub fn trace_warnings<F>(
        &mut self,
        world: Tracked<dyn World + '_>,
        make_point: F,
        span: Span,
    ) where
        F: Fn() -> Tracepoint,
    {
        self.warnings = std::mem::take(&mut self.warnings).trace(world, make_point, span);
    }

    /// Apply warning suppression.
    pub fn suppress_warnings(&mut self, world: &dyn World) {
        self.warnings.retain(|diag| {
            let Some(identifier) = &diag.identifier else {
                // Can't suppress without an identifier.
                return true;
            };

            // Only retain warnings which weren't locally suppressed where they
            // were emitted or at any of their tracepoints.
            diag.severity != Severity::Warning
                || (!check_warning_suppressed(diag.span, world, &identifier)
                    && !diag.trace.iter().any(|tracepoint| {
                        check_warning_suppressed(tracepoint.span, world, &identifier)
                    }))
        });
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
        let hash = crate::utils::hash128(&(&warning.span, &warning.message));
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

    /// Extend from another sink.
    /// Using `Sink::extend_tracked` is preferable as it avoids a call to this
    /// function if all arguments are empty, thus avoiding an unnecessary
    /// tracked call in most cases.
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

/// Checks if a given warning is suppressed given one span it has a tracepoint
/// in. If one of the ancestors of the node where the warning occurred has a
/// warning suppression decorator sibling right before it suppressing this
/// particular warning, the warning is considered suppressed.
fn check_warning_suppressed(
    span: Span,
    world: &dyn World,
    identifier: &diag::Identifier,
) -> bool {
    let Some(file) = span.id() else {
        // Don't suppress detached warnings.
        return false;
    };

    // The source must exist if a warning occurred in the file,
    // or has a tracepoint in the file.
    let source = world.source(file).unwrap();

    let search_root = source.find(span);
    let mut searched_node = search_root.as_ref();

    // Walk the parent nodes to check for a warning suppression in the
    // previous line.
    while let Some(node) = searched_node {
        if let Some(sibling) = node.prev_attached_comment() {
            if let Some(comment) = sibling.cast::<ast::LineComment>() {
                if matches!(parse_warning_suppression(comment.content()), Some(suppressed) if identifier.name() == suppressed)
                {
                    return true;
                }
            }
        }
        searched_node = node.parent();
    }

    false
}

// TODO: replace this ad-hoc solution
// Expects a comment '//! allow("identifier")
fn parse_warning_suppression(comment: &str) -> Option<&str> {
    const ALLOW_SEGMENT: &str = "! allow(\"";
    if !comment.starts_with(ALLOW_SEGMENT) {
        return None;
    }
    let after_allow = comment.get(ALLOW_SEGMENT.len()..)?.trim();
    let (suppressed_identifier, rest) = after_allow.split_once('"')?;
    if rest.trim() != ")" {
        return None;
    }

    Some(suppressed_identifier)
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
