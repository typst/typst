use std::fmt::{self, Debug, Formatter};
use std::num::NonZeroUsize;

use comemo::Tracked;
use ecow::{EcoString, eco_format};
use typst_syntax::Span;

use crate::diag::{SourceDiagnostic, warning};
use crate::engine::Engine;
use crate::foundations::{Content, IntoValue, Repr, Selector, func, repr, scope, ty};
use crate::introspection::{History, Introspect, Introspector};
use crate::layout::{Abs, Position};
use crate::model::Numbering;

/// Makes an element available in the introspector.
pub trait Locatable {}

/// Marks an element as not queriable for the user.
pub trait Unqueriable: Locatable {}

/// Marks an element as tagged in PDF files.
pub trait Tagged {}

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
    pub fn page(self, engine: &mut Engine, span: Span) -> NonZeroUsize {
        engine.introspect(PageIntrospection(self, span))
    }

    /// Returns a dictionary with the page number and the x, y position for this
    /// location. The page number starts at one and the coordinates are measured
    /// from the top-left of the page.
    ///
    /// If you only need the page number, use `page()` instead as it allows
    /// Typst to skip unnecessary work.
    #[func]
    pub fn position(self, engine: &mut Engine, span: Span) -> Position {
        engine.introspect(PositionIntrospection(self, span))
    }

    /// Returns the page numbering pattern of the page at this location. This
    /// can be used when displaying the page counter in order to obtain the
    /// local numbering. This is useful if you are building custom indices or
    /// outlines.
    ///
    /// If the page numbering is set to `{none}` at that location, this function
    /// returns `{none}`.
    #[func]
    pub fn page_numbering(self, engine: &mut Engine, span: Span) -> Option<Numbering> {
        engine.introspect(PageNumberingIntrospection(self, span))
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

/// Retrieves the exact position of an element in the document.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct PositionIntrospection(pub Location, pub Span);

impl Introspect for PositionIntrospection {
    type Output = Position;

    fn introspect(
        &self,
        _: &mut Engine,
        introspector: Tracked<Introspector>,
    ) -> Self::Output {
        introspector.position(self.0).as_paged_or_default()
    }

    fn diagnose(&self, history: &History<Self::Output>) -> SourceDiagnostic {
        format_convergence_warning(
            self.0,
            self.1,
            history,
            "positions",
            |element| eco_format!("{element} position"),
            |pos| {
                let coord = |v: Abs| repr::format_float(v.to_pt(), Some(0), false, "pt");
                eco_format!(
                    "page {} at ({}, {})",
                    pos.page,
                    coord(pos.point.x),
                    coord(pos.point.y)
                )
            },
        )
    }
}

/// Retrieves the number of the page where an element is located.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct PageIntrospection(pub Location, pub Span);

impl Introspect for PageIntrospection {
    type Output = NonZeroUsize;

    fn introspect(
        &self,
        _: &mut Engine,
        introspector: Tracked<Introspector>,
    ) -> Self::Output {
        introspector.page(self.0)
    }

    fn diagnose(&self, history: &History<Self::Output>) -> SourceDiagnostic {
        format_convergence_warning(
            self.0,
            self.1,
            history,
            "page numbers",
            |element| eco_format!("page number of the {element}"),
            |n| eco_format!("page {n}"),
        )
    }
}

/// Retrieves the numbering of the page where an element is located.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct PageNumberingIntrospection(pub Location, pub Span);

impl Introspect for PageNumberingIntrospection {
    type Output = Option<Numbering>;

    fn introspect(
        &self,
        _: &mut Engine,
        introspector: Tracked<Introspector>,
    ) -> Self::Output {
        introspector.page_numbering(self.0).cloned()
    }

    fn diagnose(&self, history: &History<Self::Output>) -> SourceDiagnostic {
        format_convergence_warning(
            self.0,
            self.1,
            history,
            "numberings",
            |element| {
                eco_format!("numbering of the page on which the {element} is located")
            },
            |numbering| eco_format!("`{}`", numbering.clone().into_value().repr()),
        )
    }
}

/// Retrieves the supplement of the page where an element is located.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct PageSupplementIntrospection(pub Location, pub Span);

impl Introspect for PageSupplementIntrospection {
    type Output = Content;

    fn introspect(
        &self,
        _: &mut Engine,
        introspector: Tracked<Introspector>,
    ) -> Self::Output {
        introspector.page_supplement(self.0)
    }

    fn diagnose(&self, history: &History<Self::Output>) -> SourceDiagnostic {
        format_convergence_warning(
            self.0,
            self.1,
            history,
            "supplements",
            |element| {
                eco_format!("supplement of the page on which the {element} is located")
            },
            |supplement| eco_format!("`{}`", supplement.repr()),
        )
    }
}

/// The warning when an introspection on a [`Location`] did not converge.
fn format_convergence_warning<T>(
    loc: Location,
    span: Span,
    history: &History<T>,
    output_kind_plural: &str,
    format_output_kind: impl FnOnce(&str) -> EcoString,
    format_output: impl FnMut(&T) -> EcoString,
) -> SourceDiagnostic {
    let elem = history.final_introspector().query_first(&Selector::Location(loc));
    let kind = match &elem {
        Some(content) => content.elem().name(),
        None => "element",
    };

    let what = format_output_kind(kind);
    let mut diag = warning!(span, "{what} did not stabilize");

    if let Some(elem) = elem
        && !elem.span().is_detached()
    {
        diag.spanned_hint(eco_format!("{kind} was created here"), elem.span());
    }

    diag.with_hint(history.hint(output_kind_plural, format_output))
}
