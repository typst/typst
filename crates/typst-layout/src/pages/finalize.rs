use typst_library::diag::SourceResult;
use typst_library::engine::Engine;
use typst_library::introspection::{ManualPageCounter, Tag};
use typst_library::layout::{Frame, FrameItem, Page, Point};

use super::LayoutedPage;

/// Piece together the inner page frame and the marginals. We can only do this
/// at the very end because inside/outside margins require knowledge of the
/// physical page number, which is unknown during parallel layout.
pub fn finalize(
    engine: &mut Engine,
    counter: &mut ManualPageCounter,
    tags: &mut Vec<Tag>,
    LayoutedPage {
        inner,
        mut margin,
        bleed,
        binding,
        two_sided,
        header,
        footer,
        background,
        foreground,
        fill,
        numbering,
        supplement,
    }: LayoutedPage,
) -> SourceResult<Page> {
    // If two sided, left becomes inside and right becomes outside.
    // Thus, for left-bound pages, we want to swap on even pages and
    // for right-bound pages, we want to swap on odd pages.
    if two_sided && binding.swap(counter.physical()) {
        std::mem::swap(&mut margin.left, &mut margin.right);
    }

    // Create a frame for the full page.
    let mut frame =
        Frame::hard(inner.size() + margin.sum_by_axis() + bleed.sum_by_axis());

    // Add tags.
    for tag in tags.drain(..) {
        frame.push(Point::zero(), FrameItem::Tag(tag));
    }

    let content_origin = Point::new(bleed.left, bleed.top);

    // Add the "before" marginals. The order in which we push things here is
    // important as it affects the relative ordering of introspectable elements
    // and thus how counters resolve.
    if let Some(background) = background {
        frame.push_frame(Point::zero(), background);
    }
    if let Some(header) = header {
        frame.push_frame(content_origin + Point::with_x(margin.left), header);
    }

    // Add the inner contents.
    frame.push_frame(content_origin + Point::new(margin.left, margin.top), inner);

    // Add the "after" marginals.
    if let Some(footer) = footer {
        let y = frame.height() - footer.height() - bleed.bottom;
        frame.push_frame(Point::new(margin.left + bleed.left, y), footer);
    }
    if let Some(foreground) = foreground {
        frame.push_frame(Point::zero(), foreground);
    }

    // Apply counter updates from within the page to the manual page counter.
    counter.visit(engine, &frame)?;

    // Get this page's number and then bump the counter for the next page.
    let number = counter.logical();
    counter.step();

    Ok(Page { frame, bleed, fill, numbering, supplement, number })
}
