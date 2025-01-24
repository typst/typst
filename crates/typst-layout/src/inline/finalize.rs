use typst_library::introspection::SplitLocator;
use typst_utils::Numeric;

use super::*;

/// Turns the selected lines into frames.
#[typst_macros::time]
pub fn finalize(
    engine: &mut Engine,
    p: &Preparation,
    lines: &[Line],
    styles: StyleChain,
    region: Size,
    expand: bool,
    locator: &mut SplitLocator<'_>,
) -> SourceResult<Fragment> {
    // Determine the resulting width: Full width of the region if we should
    // expand or there's fractional spacing, fit-to-width otherwise.
    let width = if !region.x.is_finite()
        || (!expand && lines.iter().all(|line| line.fr().is_zero()))
    {
        region
            .x
            .min(p.hang + lines.iter().map(|line| line.width).max().unwrap_or_default())
    } else {
        region.x
    };

    // Stack the lines into one frame per region.
    lines
        .iter()
        .map(|line| commit(engine, p, line, width, region.y, locator, styles))
        .collect::<SourceResult<_>>()
        .map(Fragment::frames)
}
