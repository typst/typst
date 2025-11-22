use ecow::EcoVec;
use typst_library::diag::SourceResult;
use typst_library::engine::Engine;
use typst_library::foundations::{Packed, StyleChain};
use typst_library::introspection::Locator;
use typst_library::math::*;

use crate::HtmlNode;

pub(crate) fn convert_math_to_nodes(
    elem: &Packed<EquationElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
) -> SourceResult<EcoVec<HtmlNode>> {
    let mut locator = locator.split();
    let resolver = MathResolver::new();
    let _run = resolver.resolve(elem, engine, &mut locator, styles)?;

    Ok(EcoVec::new())
}
