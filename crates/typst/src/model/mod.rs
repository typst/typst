//! The document model.

mod content;
mod element;
mod introspect;
mod label;
mod realize;
mod selector;
mod styles;

#[doc(inline)]
pub use typst_macros::element;

pub use self::content::{Content, MetaElem, PlainText};
pub use self::element::{Construct, ElemFunc, Element, NativeElemFunc, Set};
pub use self::introspect::{Introspector, Location, Locator};
pub use self::label::{Label, Unlabellable};
pub use self::realize::{
    applicable, realize, Behave, Behaviour, Finalize, Guard, Locatable, Show, Synthesize,
};
pub use self::selector::{LocatableSelector, Selector, ShowableSelector};
pub use self::styles::{
    Fold, Property, Recipe, Resolve, Style, StyleChain, StyleVec, StyleVecBuilder,
    Styles, Transform,
};

use std::mem::ManuallyDrop;

use comemo::{Track, Tracked, TrackedMut, Validate};

use crate::diag::{SourceError, SourceResult};
use crate::doc::Document;
use crate::eval::Tracer;
use crate::World;

/// Typeset content into a fully layouted document.
#[comemo::memoize]
#[tracing::instrument(skip(world, tracer, content))]
pub fn typeset(
    world: Tracked<dyn World + '_>,
    mut tracer: TrackedMut<Tracer>,
    content: &Content,
) -> SourceResult<Document> {
    tracing::info!("Starting typesetting");

    let library = world.library();
    let styles = StyleChain::new(&library.styles);

    let mut iter = 0;
    let mut document;
    let mut delayed;

    // We need `ManuallyDrop` until this lands in stable:
    // https://github.com/rust-lang/rust/issues/70919
    let mut introspector = ManuallyDrop::new(Introspector::new(&[]));

    // Relayout until all introspections stabilize.
    // If that doesn't happen within five attempts, we give up.
    loop {
        tracing::info!("Layout iteration {iter}");

        delayed = DelayedErrors::default();

        let constraint = <Introspector as Validate>::Constraint::new();
        let mut locator = Locator::new();
        let mut vt = Vt {
            world,
            tracer: TrackedMut::reborrow_mut(&mut tracer),
            locator: &mut locator,
            introspector: introspector.track_with(&constraint),
            delayed: delayed.track_mut(),
        };

        // Layout!
        let result = (library.items.layout)(&mut vt, content, styles)?;

        // Drop the old introspector.
        ManuallyDrop::into_inner(introspector);

        // Only now assign the document and construct the new introspector.
        document = result;
        introspector = ManuallyDrop::new(Introspector::new(&document.pages));
        iter += 1;

        if iter >= 5 || introspector.validate(&constraint) {
            break;
        }
    }

    // Drop the introspector.
    ManuallyDrop::into_inner(introspector);

    // Promote delayed errors.
    if !delayed.0.is_empty() {
        return Err(Box::new(delayed.0));
    }

    Ok(document)
}

/// A virtual typesetter.
///
/// Holds the state needed to [typeset] content.
pub struct Vt<'a> {
    /// The compilation environment.
    pub world: Tracked<'a, dyn World + 'a>,
    /// Provides access to information about the document.
    pub introspector: Tracked<'a, Introspector>,
    /// Provides stable identities to elements.
    pub locator: &'a mut Locator<'a>,
    /// Delayed errors that do not immediately terminate execution.
    pub delayed: TrackedMut<'a, DelayedErrors>,
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
                for error in *errors {
                    self.delayed.push(error);
                }
                T::default()
            }
        }
    }
}

/// Holds delayed errors.
#[derive(Default, Clone)]
pub struct DelayedErrors(Vec<SourceError>);

#[comemo::track]
impl DelayedErrors {
    /// Push a delayed error.
    fn push(&mut self, error: SourceError) {
        self.0.push(error);
    }
}
