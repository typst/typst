use ecow::EcoVec;
use typst_library::diag::SourceResult;
use typst_library::engine::Engine;
use typst_library::math::ir::MathItem;

use crate::HtmlNode;

pub(crate) fn convert_math_to_nodes(
    _item: MathItem,
    _engine: &mut Engine,
) -> SourceResult<EcoVec<HtmlNode>> {
    Ok(EcoVec::new())
}
