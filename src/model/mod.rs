//! The document model.

mod content;
mod element;
mod introspect;
mod realize;
mod styles;

pub use typst_macros::element;

pub use self::content::*;
pub use self::element::*;
pub use self::introspect::*;
pub use self::realize::*;
pub use self::styles::*;

use std::mem::ManuallyDrop;

use comemo::{Track, Tracked, TrackedMut, Validate};

use crate::diag::SourceResult;
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

    let mut document;
    let mut iter = 0;

    // We need `ManuallyDrop` until this lands in stable:
    // https://github.com/rust-lang/rust/issues/70919
    let mut introspector = ManuallyDrop::new(Introspector::new(&[]));

    // Relayout until all introspections stabilize.
    // If that doesn't happen within five attempts, we give up.
    loop {
        tracing::info!("Layout iteration {iter}");

        let constraint = <Introspector as Validate>::Constraint::new();
        let mut locator = Locator::new();
        let mut vt = Vt {
            world,
            tracer: TrackedMut::reborrow_mut(&mut tracer),
            locator: &mut locator,
            introspector: introspector.track_with(&constraint),
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

    Ok(document)
}

/// A virtual typesetter.
///
/// Holds the state needed to [typeset] content.
pub struct Vt<'a> {
    /// The compilation environment.
    pub world: Tracked<'a, dyn World + 'a>,
    /// The tracer for inspection of the values an expression produces.
    pub tracer: TrackedMut<'a, Tracer>,
    /// Provides stable identities to elements.
    pub locator: &'a mut Locator<'a>,
    /// Provides access to information about the document.
    pub introspector: Tracked<'a, Introspector>,
}
