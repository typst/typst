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

    // Keep track of active links across lines, as some links might span
    // multiple lines.
    //
    // The list of active links starts with those that began before the
    // paragraph started. Guaranteed to be start events, but pushed in the order
    // of end events (as unmatched end events are found), so appropriately
    // reverse their order below.
    let mut active_links = p
        .initial_events
        .iter()
        .rev()
        .map(|evt| match evt {
            Event::StartLink(dest) => dest,
            Event::EndLink(_) => unreachable!(),
        })
        .collect::<Vec<_>>();


    // Stack the lines into one frame per region.
    lines
        .iter()
        .map(|line| commit(engine, p, line, width, region.y, locator, &mut active_links))
        .collect::<SourceResult<_>>()
        .map(Fragment::frames)
}
