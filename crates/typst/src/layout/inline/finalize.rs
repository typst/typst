use super::*;
use crate::layout::{Abs, Frame, Point};
use crate::utils::Numeric;

/// Turns the selected lines into frames.
#[typst_macros::time]
pub fn finalize(
    engine: &mut Engine,
    p: &Preparation,
    lines: &[Line],
    styles: StyleChain,
    region: Size,
    expand: bool,
) -> SourceResult<Fragment> {
    // Determine the paragraph's width: Full width of the region if we should
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
    let shrink = ParElem::shrink_in(styles);
    let mut frames: Vec<Frame> = lines
        .iter()
        .map(|line| commit(engine, p, line, width, region.y, shrink))
        .collect::<SourceResult<_>>()?;

    // Positive ratios enable prevention, while zero and negative ratios disable
    // it.
    if p.costs.orphan().get() > 0.0 {
        // Prevent orphans.
        if frames.len() >= 2 && !frames[1].is_empty() {
            let second = frames.remove(1);
            let first = &mut frames[0];
            merge(first, second, p.leading);
        }
    }
    if p.costs.widow().get() > 0.0 {
        // Prevent widows.
        let len = frames.len();
        if len >= 2 && !frames[len - 2].is_empty() {
            let second = frames.pop().unwrap();
            let first = frames.last_mut().unwrap();
            merge(first, second, p.leading);
        }
    }

    Ok(Fragment::frames(frames))
}

/// Merge two line frames
fn merge(first: &mut Frame, second: Frame, leading: Abs) {
    let offset = first.height() + leading;
    let total = offset + second.height();
    first.push_frame(Point::with_y(offset), second);
    first.size_mut().y = total;
}
