use typst_syntax::Span;

use crate::foundations::{Content, NativeElement, elem, func};
use crate::math::Mathy;

/// A square root.
///
/// ```example
/// $ sqrt(3 - 2 sqrt(2)) = sqrt(2) - 1 $
/// ```
#[func(title = "Square Root")]
pub fn sqrt(
    span: Span,
    /// The expression to take the square root of.
    radicand: Content,
) -> Content {
    RootElem::new(radicand).pack().spanned(span)
}

/// A general root.
///
/// ```example
/// $ root(3, x) $
/// ```
#[elem(Mathy)]
pub struct RootElem {
    /// Which root of the radicand to take.
    #[positional]
    pub index: Option<Content>,

    /// The expression to take the root of.
    #[required]
    pub radicand: Content,
}
