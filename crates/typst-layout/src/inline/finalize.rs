use typst_library::introspection::SplitLocator;
use typst_utils::Numeric;

use super::*;

/// Turns the selected lines into frames.
#[typst_macros::time]
pub fn finalize(
    engine: &mut Engine,
    p: &Preparation,
    lines: &[Line],
    region: Size,
    expand: bool,
    locator: &mut SplitLocator<'_>,
) -> SourceResult<Fragment> {
    let flow_len = region.axis_length(p.config.dir.axis());
    let cross_len = region.axis_length(p.config.dir.axis().other());

    // Determine the resulting length: Full flow dimension of the region if we should
    // expand or there's fractional spacing, fit-to-length otherwise.
    let length = if !flow_len.is_finite()
        || (!expand && lines.iter().all(|line| line.fr().is_zero()))
    {
        flow_len.min(
            p.config.hanging_indent
                + lines.iter().map(|line| line.length).max().unwrap_or_default(),
        )
    } else {
        flow_len
    };

    // Stack the lines into one frame per region.
    lines
        .iter()
        .map(|line| commit(engine, p, line, length, cross_len, locator))
        .collect::<SourceResult<_>>()
        .map(Fragment::frames)
}
