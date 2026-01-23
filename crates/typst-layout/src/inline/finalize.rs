use typst_library::introspection::SplitLocator;
use typst_utils::Numeric;

use super::*;

/// Turns the selected lines into frames or inline blocks.
#[typst_macros::time]
pub fn finalize(
    engine: &mut Engine,
    p: &Preparation,
    lines: &[Line],
    region: Size,
    expand: bool,
    locator: &mut SplitLocator<'_>,
) -> SourceResult<Vec<ParChild>> {
    // Determine the resulting width: Full width of the region if we should
    // expand or there's fractional spacing, fit-to-width otherwise.
    let width = if !region.x.is_finite()
        || (!expand && lines.iter().all(|line| line.fr().is_zero()))
    {
        region.x.min(
            p.config.hanging_indent
                + lines.iter().map(|line| line.width).max().unwrap_or_default(),
        )
    } else {
        region.x
    };

    lines
        .iter()
        .map(|line| commit(engine, p, line, width, region.y, locator))
        .collect()
}
