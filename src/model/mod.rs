//! The document model.

mod content;
mod element;
mod introspect;
mod realize;
mod styles;

pub use self::content::*;
pub use self::element::*;
pub use self::introspect::*;
pub use self::realize::*;
pub use self::styles::*;

pub use typst_macros::element;

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
    tracing::info!("Starting layout");
    let library = world.library();
    let styles = StyleChain::new(&library.styles);

    let mut document;
    let mut iter = 0;
    let mut introspector = Introspector::new(&[]);

    // Relayout until all introspections stabilize.
    // If that doesn't happen within five attempts, we give up.
    loop {
        tracing::info!("Layout iteration {iter}");

        let mut provider = StabilityProvider::new();
        let constraint = <Introspector as Validate>::Constraint::new();
        let mut vt = Vt {
            world,
            tracer: TrackedMut::reborrow_mut(&mut tracer),
            provider: provider.track_mut(),
            introspector: introspector.track_with(&constraint),
        };

        document = (library.items.layout)(&mut vt, content, styles)?;
        iter += 1;

        introspector = Introspector::new(&document.pages);

        if iter >= 5 || introspector.validate(&constraint) {
            break;
        }
    }

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
    pub provider: TrackedMut<'a, StabilityProvider>,
    /// Provides access to information about the document.
    pub introspector: Tracked<'a, Introspector>,
}

impl Vt<'_> {
    /// Mutably reborrow with a shorter lifetime.
    pub fn reborrow_mut(&mut self) -> Vt<'_> {
        Vt {
            world: self.world,
            tracer: TrackedMut::reborrow_mut(&mut self.tracer),
            provider: TrackedMut::reborrow_mut(&mut self.provider),
            introspector: self.introspector,
        }
    }
}
