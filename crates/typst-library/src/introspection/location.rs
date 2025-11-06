use std::fmt::{self, Debug, Formatter};
use std::num::NonZeroUsize;

use ecow::EcoString;

use crate::engine::Engine;
use crate::foundations::{Repr, func, scope, ty};
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
/// Elements that are automatically assigned a location are called _locatable._
/// For efficiency reasons, not all elements are locatable.
///
/// - In the [Model category]($category/model), most elements are locatable.
///   This is because semantic elements like [headings]($heading) and
///   [figures]($figure) are often used with introspection.
///
/// - In the [Text category]($category/text), the [`raw`] element, and the
///   decoration elements [`underline`], [`overline`], [`strike`], and
///   [`highlight`] are locatable as these are also quite semantic in nature.
///
/// - In the [Introspection category]($category/introspection), the [`metadata`]
///   element is locatable as being queried for is its primary purpose.
///
/// - In the other categories, most elements are not locatable. Exceptions are
///   [`math.equation`] and [`image`].
///
/// To find out whether a specific element is locatable, you can try to
/// [`query`] for it.
///
/// Note that you can still observe elements that are not locatable in queries
/// through other means, for instance, when they have a label attached to them.
#[ty(scope)]
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Location(u128);

impl Location {
    /// Create a new location from a unique hash.
    pub fn new(hash: u128) -> Self {
        Self(hash)
    }

    /// Extract the raw hash.
    pub fn hash(self) -> u128 {
        self.0
    }

    /// Produces a well-known variant of this location.
    ///
    /// This is a synthetic location created from another one and is used, for
    /// example, in bibliography management to create individual linkable
    /// locations for reference entries from the bibliography's location.
    pub fn variant(self, n: usize) -> Self {
        Self(typst_utils::hash128(&(self.0, n)))
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
    /// If the page numbering is set to `{none}` at that location, this function
    /// returns `{none}`.
    #[func]
    pub fn page_numbering(self, engine: &mut Engine) -> Option<Numbering> {
        engine.introspector.page_numbering(self).cloned()
    }
}

impl Debug for Location {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if f.alternate() {
            write!(f, "Location({})", self.0)
        } else {
            // Print a shorter version by default to make it more readable.
            let truncated = self.0 as u16;
            write!(f, "Location({truncated})")
        }
    }
}

impl Repr for Location {
    fn repr(&self) -> EcoString {
        "location(..)".into()
    }
}

/// Can be used to have a location as a key in an ordered set or map.
///
/// [`Location`] itself does not implement [`Ord`] because comparing hashes like
/// this has no semantic meaning. The potential for misuse (e.g. checking
/// whether locations have a particular relative ordering) is relatively high.
///
/// Still, it can be useful to have orderable locations for things like sets.
/// That's where this type comes in.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct LocationKey(u128);

impl LocationKey {
    /// Create a location key from a location.
    pub fn new(location: Location) -> Self {
        Self(location.0)
    }
}

impl From<Location> for LocationKey {
    fn from(location: Location) -> Self {
        Self::new(location)
    }
}

/// Make this element available in the introspector.
pub trait Locatable {}

/// Make this element not queriable for the user.
pub trait Unqueriable: Locatable {}

/// Marks this element as tagged in PDF files.
pub trait Tagged {}
