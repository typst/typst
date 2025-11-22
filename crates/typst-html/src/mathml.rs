use ecow::EcoVec;
use typst_library::diag::SourceResult;
use typst_library::engine::Engine;
use typst_library::foundations::{Packed, StyleChain};
use typst_library::introspection::Locator;
use typst_library::math::EquationElem;
use typst_library::math::ir::resolve_equation;
use typst_library::routines::Arenas;

use crate::HtmlNode;

pub(crate) fn convert_math_to_nodes(
    elem: &Packed<EquationElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
) -> SourceResult<EcoVec<HtmlNode>> {
    let mut locator = locator.split();

    let arenas = Arenas::default();
    let _item = resolve_equation(elem, engine, &mut locator, &arenas, styles)?;

    Ok(EcoVec::new())
}
