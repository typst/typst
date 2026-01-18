use typst_library::introspection::SplitLocator;
use typst_library::layout::ParExclusions;
use typst_utils::Numeric;

use super::*;

/// Turns the selected lines into frames.
///
/// # Arguments
/// * `exclusions` - Optional exclusion zones for wrap-floats. When provided,
///   lines adjacent to left-aligned floats will be shifted right by the
///   appropriate amount.
/// * `leading` - Space between lines (only used when exclusions are present).
#[typst_macros::time]
pub fn finalize(
    engine: &mut Engine,
    p: &Preparation,
    lines: &[Line],
    region: Size,
    expand: bool,
    locator: &mut SplitLocator<'_>,
    exclusions: Option<&ParExclusions>,
    leading: Abs,
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

    // If we have exclusions, compute per-line x-offsets based on y-position.
    // Otherwise, use the simple fast path with no offsets.
    if let Some(excl) = exclusions {
        finalize_with_exclusions(engine, p, lines, width, region.y, locator, excl, leading)
    } else {
        // Fast path: no exclusions, all lines at x=0
        lines
            .iter()
            .map(|line| commit(engine, p, line, width, region.y, locator, Abs::zero()))
            .collect::<SourceResult<_>>()
            .map(Fragment::frames)
    }
}

/// Finalize with exclusion zones, computing per-line x-offsets.
fn finalize_with_exclusions(
    engine: &mut Engine,
    p: &Preparation,
    lines: &[Line],
    width: Abs,
    full: Abs,
    locator: &mut SplitLocator<'_>,
    exclusions: &ParExclusions,
    leading: Abs,
) -> SourceResult<Fragment> {
    let mut y = Abs::zero();
    let mut frames = Vec::with_capacity(lines.len());

    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            y += leading;
        }

        // Get the left x-offset for this line's y-position from exclusions.
        let left_x_offset = exclusions.left_offset(y);

        // Get the available width for this line, accounting for exclusions.
        // This is crucial for justified text: justification stretches to fill
        // the available width, so we must use the reduced width, not full width.
        let line_width = exclusions.available_width(width, y);

        let frame = commit(engine, p, line, line_width, full, locator, left_x_offset)?;
        y += frame.height();
        frames.push(frame);
    }

    Ok(Fragment::frames(frames))
}
