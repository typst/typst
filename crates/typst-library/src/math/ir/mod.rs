//! Intermediate representation for math.

mod item;
mod preprocess;
mod resolve;

pub use self::item::*;

use self::resolve::MathResolver;
use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{Packed, StyleChain};
use crate::introspection::SplitLocator;
use crate::math::EquationElem;
use crate::routines::Arenas;

/// Resolves an equation's body into a [`MathItem`].
///
/// The returned `MathItem` has the same lifetime as the provided arenas.
#[typst_macros::time(name = "math ir creation")]
pub fn resolve_equation<'a>(
    elem: &'a Packed<EquationElem>,
    engine: &mut Engine,
    locator: &mut SplitLocator<'a>,
    arenas: &'a Arenas,
    styles: StyleChain<'a>,
) -> SourceResult<MathItem<'a>> {
    let mut context = MathResolver::new(engine, locator, arenas);
    context.resolve_into_item(&elem.body, styles)
}
