use std::sync::LazyLock;

use ecow::{EcoString, eco_format};
use typst_library::diag::SourceResult;
use typst_library::engine::Engine;
use typst_library::foundations::Content;
use typst_library::math::ir::MathItem;

pub(crate) static EQUATION_CSS_STYLES: LazyLock<EcoString> =
    LazyLock::new(|| eco_format!(""));

pub(crate) fn convert_math_to_nodes(
    _item: MathItem,
    _engine: &mut Engine,
    _block: bool,
) -> SourceResult<Vec<Content>> {
    Ok(Vec::new())
}
