use std::any::{Any, TypeId};
use std::fmt::{Debug, Write};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use comemo::{Track, Tracked};
use ecow::{EcoString, EcoVec, eco_format};
use typst_syntax::Span;
use typst_utils::Protected;

use crate::World;
use crate::diag::{SourceDiagnostic, warning};
use crate::engine::{Engine, Route, Sink, Traced};
use crate::introspection::Introspector;
use crate::routines::Routines;

pub const MAX_ITERS: usize = 5;
pub const ITER_NAMES: &[&str] =
    &["iter (1)", "iter (2)", "iter (3)", "iter (4)", "iter (5)"];

const INSTANCES: usize = MAX_ITERS + 1;

/// Analyzes all introspections that were performed during compilation and
/// produces non-convergence diagnostics.
#[typst_macros::time(name = "analyze introspections")]
pub fn analyze(
    world: Tracked<dyn World + '_>,
    routines: &Routines,
    introspectors: [&Introspector; INSTANCES],
    introspections: &[Introspection],
) -> EcoVec<SourceDiagnostic> {
    let mut sink = Sink::new();
    for introspection in introspections {
        if let Some(warning) = introspection.0.diagnose(world, routines, introspectors) {
            sink.warn(warning);
        }
    }

    // Let's say you want to write some code that depends on the presence of an
    // element in the document. You could (1) use the `QueryIntrospection` and
    // then do your emptyness check afterwards or you could (2) write a new
    // [introspection](Introspect) that queries internally and returns a
    // boolean.
    //
    // In case (1), the introspection observes the same data as comemo. Thus,
    // the introspection will have converged if and only if comemo validation
    // also passed.
    //
    // However, in case (2) the introspection is filtering out data that comemo
    // did observe. Thus, the validation may fail but the document actually did
    // converge! In this case, we reach `analyze` (because comemo validation
    // failed), but we get zero diagnostics. In this case, we do _not_ issue the
    // convergence warning, since the document did in fact converge.
    //
    // Note that we could also entirely decouple convergence checks from comemo.
    // However, it's nice to have as a fast path as the comemo checks are more
    // lightweight.
    let mut diags = sink.warnings();
    if !diags.is_empty() {
        let summary = warning!(
            Span::detached(),
            "document did not converge within five attempts";
            hint: "see {} additional warning{} for more details",
                diags.len(),
                if diags.len() > 1 { "s" } else { "" };
            hint: "see https://typst.app/help/convergence for help";
        );
        diags.insert(0, summary);
    }

    diags
}

/// An inquiry for retrieving a piece of information from the document.
///
/// This includes queries, counter retrievals, and various other things that can
/// be observed on a finished document.
///
/// Document iteration N+1 observes the `Output` values from the document built
/// by iteration N. If the output values do not stabilize by the iteration
/// limit, a non-convergence warning will be created via
/// [`diagnose`](Self::diagnose).
///
/// Some introspections directly map to functions on the introspector while
/// others are more high-level. To decide between these two options, think about
/// how you want a non-convergence diagnostic to look. If the diagnostic for the
/// generic introspection (e.g. `QueryIntrospection`) is sufficiently clear, you
/// can use that one directly. If you'd rather fine-tune the diagnostic for
/// non-convergence, create a new introspection.
///
/// A good example of this are counters and state: They could just use
/// `QueryIntrospection`, but are custom introspections so that the diagnostics
/// can expose non-convergence in a way that's closer to what the user operates
/// with.
pub trait Introspect: Debug + PartialEq + Hash + Send + Sync + Sized + 'static {
    /// The kind of output the introspection produces. This is what should
    /// stabilize.
    ///
    /// Note that how much information you have in the output may affect
    /// convergence behavior. For instance, if you reduce down the result of a
    /// query introspection to a boolean specifying whether the query yielded at
    /// least one element, this may converge one iteration sooner than a raw
    /// query would have (even if you always reduce the query to the same bool
    /// externally).
    ///
    /// Thus, it matters whether this reduction is performed as part of the
    /// introspection or externally. This is similar to how `location.page()`
    /// may converge one iteration sooner than `location.position().page`.
    /// Consider this example:
    ///
    /// ```typ
    /// #switch(n => if n == 5 {
    ///   v(1cm)
    ///   _ = locate(heading).page()
    /// })
    /// = Heading
    /// ```
    ///
    /// Both will always result in the same output, but observing the X/Y
    /// position may end up requiring one extra iteration and if this happens
    /// exactly at the limit of five iterations, the warning may appear (without
    /// any effect on the document, which did actually converge).
    ///
    /// In theory, we could detect this scenario by compiling one more time and
    /// ensuring the document is _exactly_ the same. For now, we're not doing
    /// this, but it's an option.
    type Output: Hash;

    /// Resolves the output value.
    ///
    /// Will primarily use the `introspector`, but is passed the full engine in
    /// case user functions need to be called (as is the case for counters).
    fn introspect(
        &self,
        engine: &mut Engine,
        introspector: Tracked<Introspector>,
    ) -> Self::Output;

    /// Produces a diagnostic for non-convergence given the history of its
    /// output values.
    fn diagnose(&self, history: &History<Self::Output>) -> SourceDiagnostic;
}

/// A type-erased representation of an [introspection](Introspect) that was
/// recorded during compilation.
#[derive(Debug, Clone, Hash)]
#[allow(clippy::derived_hash_with_manual_eq)]
pub struct Introspection(Arc<dyn Bounds>);

impl Introspection {
    /// Type erase a strongly-typed introspection.
    pub fn new<I>(inner: I) -> Self
    where
        I: Introspect,
    {
        Self(Arc::new(inner))
    }
}

impl PartialEq for Introspection {
    fn eq(&self, other: &Self) -> bool {
        self.0.dyn_eq(other)
    }
}

trait Bounds: Debug + Send + Sync + Any + 'static {
    fn diagnose(
        &self,
        world: Tracked<dyn World + '_>,
        routines: &Routines,
        introspectors: [&Introspector; INSTANCES],
    ) -> Option<SourceDiagnostic>;
    fn dyn_eq(&self, other: &Introspection) -> bool;
    fn dyn_hash(&self, state: &mut dyn Hasher);
}

impl<T> Bounds for T
where
    T: Introspect,
{
    fn diagnose(
        &self,
        world: Tracked<dyn World + '_>,
        routines: &Routines,
        introspectors: [&Introspector; INSTANCES],
    ) -> Option<SourceDiagnostic> {
        let history =
            History::compute(world, routines, introspectors, |engine, introspector| {
                self.introspect(engine, introspector)
            });
        (!history.converged()).then(|| self.diagnose(&history))
    }

    fn dyn_eq(&self, other: &Introspection) -> bool {
        let Some(other) = (&*other.0 as &dyn Any).downcast_ref::<Self>() else {
            return false;
        };
        self == other
    }

    fn dyn_hash(&self, mut state: &mut dyn Hasher) {
        // Also hash the TypeId since introspections with different types but
        // equal data should be different.
        TypeId::of::<Self>().hash(&mut state);
        self.hash(&mut state);
    }
}

impl Hash for dyn Bounds {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.dyn_hash(state);
    }
}

/// A history of values that were observed throughout iterations, alongside the
/// introspectors they were observed for.
pub struct History<'a, T>([(&'a Introspector, T); INSTANCES]);

impl<'a, T> History<'a, T> {
    /// Computes the value for each introspector with an ad-hoc engine.
    fn compute(
        world: Tracked<dyn World + '_>,
        routines: &Routines,
        introspectors: [&'a Introspector; INSTANCES],
        f: impl Fn(&mut Engine, Tracked<'a, Introspector>) -> T,
    ) -> Self {
        Self(introspectors.map(|introspector| {
            let tracked = introspector.track();
            let traced = Traced::default();
            let mut sink = Sink::new();
            let mut engine = Engine {
                world,
                introspector: Protected::new(tracked),
                traced: traced.track(),
                sink: sink.track_mut(),
                route: Route::default(),
                routines,
            };
            (introspector, f(&mut engine, tracked))
        }))
    }

    /// Whether the values in this history converged, i.e. the final and
    /// pre-final values are the same.
    pub fn converged(&self) -> bool
    where
        T: Hash,
    {
        // We compare by hash because the values should be fully the same, i.e.
        // with no observable difference. When a state changes from `0.0`
        // (float) to `0` (int), that could be observed in the next iteration
        // and should not count as converged.
        typst_utils::hash128(&self.0[MAX_ITERS - 1].1)
            == typst_utils::hash128(&self.0[MAX_ITERS].1)
    }

    /// Transforms the contained values with `f`.
    pub fn map<U>(self, mut f: impl FnMut(T) -> U) -> History<'a, U> {
        History(self.0.map(move |(i, t)| (i, f(t))))
    }

    /// Takes a reference to the contained values.
    pub fn as_ref(&self) -> History<'a, &T> {
        History(self.0.each_ref().map(|(i, t)| (*i, t)))
    }

    /// Accesses the final iteration's introspector.
    pub fn final_introspector(&self) -> &'a Introspector {
        self.0[MAX_ITERS].0
    }

    /// Produces a hint with the observed values for each iteration.
    pub fn hint(&self, what: &str, mut f: impl FnMut(&T) -> EcoString) -> EcoString {
        let mut hint = eco_format!("the following {what} were observed:");
        for (i, (_, val)) in self.0.iter().enumerate() {
            let attempt = match i {
                0..MAX_ITERS => eco_format!("run {}", i + 1),
                MAX_ITERS => eco_format!("final"),
                _ => panic!(),
            };
            let output = f(val);
            write!(hint, "\n- {attempt}: {output}").unwrap();
        }
        hint
    }
}
