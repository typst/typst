use std::num::NonZeroUsize;

use ecow::EcoString;

use crate::engine::Engine;
use crate::foundations::{func, scope, ty, Repr};
use crate::layout::Position;
use crate::model::Numbering;

/// Identifies an element in the document.
///
/// A location uniquely identifies an element in the document and lets you
/// access its absolute position on the pages. You can retrieve the current
/// location with the [`here`] function and the location of a queried or shown
/// element with the [`location()`]($content.location) method on content.
///
/// # Locatable elements { #locatable }
/// Currently, only a subset of element functions is locatable. Aside from
/// headings and figures, this includes equations, references and all elements
/// with an explicit label. As a result, you _can_ query for e.g. [`strong`]
/// elements, but you will find only those that have an explicit label attached
/// to them. This limitation will be resolved in the future.
#[ty(scope)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Location {
    /// The hash of the element.
    pub hash: u128,
    /// An unique number among elements with the same hash. This is the reason
    /// we need a `Locator` everywhere.
    pub disambiguator: usize,
    /// A synthetic location created from another one. This is used for example
    /// in bibliography management to create individual linkable locations for
    /// reference entries from the bibliography's location.
    pub variant: usize,
}

impl Location {
    /// Produce a variant of this location.
    pub fn variant(mut self, n: usize) -> Self {
        self.variant = n;
        self
    }
}

#[scope]
impl Location {
    /// Returns the page number for this location.
    ///
    /// Note that this does not return the value of the [page counter]($counter)
    /// at this location, but the true page number (starting from one).
    ///
    /// If you want to know the value of the page counter, use
    /// `{counter(page).at(loc)}` instead.
    ///
    /// Can be used with [`here`] to retrieve the physical page position
    /// of the current context:
    /// ```example
    /// #context [
    ///   I am located on
    ///   page #here().page()
    /// ]
    /// ```
    #[func]
    pub fn page(self, engine: &mut Engine) -> NonZeroUsize {
        engine.introspector.page(self)
    }

    /// Returns a dictionary with the page number and the x, y position for this
    /// location. The page number starts at one and the coordinates are measured
    /// from the top-left of the page.
    ///
    /// If you only need the page number, use `page()` instead as it allows
    /// Typst to skip unnecessary work.
    #[func]
    pub fn position(self, engine: &mut Engine) -> Position {
        engine.introspector.position(self)
    }

    /// Returns the page numbering pattern of the page at this location. This
    /// can be used when displaying the page counter in order to obtain the
    /// local numbering. This is useful if you are building custom indices or
    /// outlines.
    ///
    /// If the page numbering is set to `none` at that location, this function
    /// returns `none`.
    #[func]
    pub fn page_numbering(self, engine: &mut Engine) -> Option<Numbering> {
        engine.introspector.page_numbering(self).cloned()
    }
}

impl Repr for Location {
    fn repr(&self) -> EcoString {
        "..".into()
    }
}

/// Makes this element locatable through `engine.locate`.
pub trait Locatable {}
